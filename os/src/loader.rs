use crate::mm::addr::*;
use crate::mm::aspace::{AddressSpace, KERNEL_ASPACE, KSTACK_BASE_VPN, TRAMPOLINE_BASE_VPN};
use crate::mm::vmo::{VMObjectPaged, VMObjectPhysical, TRAMPOLINE};
use crate::mm::PTEFlags;
use crate::task::TaskContext;
use crate::trap::context::TrapContext;
use alloc::sync::Arc;
use core::slice;
use lazy_static::*;

global_asm!(include_str!("embed_app.asm"));

pub const USER_STACK_SIZE: usize = 4096 * 16;
pub const KERNEL_STACK_SIZE: usize = 4096 * 16;
pub const MAX_APP_NUM: usize = 16;
lazy_static! {
    pub static ref APP_BASE_ADDRESS: usize = option_env!("USER_BASE_ADDRESS_START")
        .and_then(|s| usize::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x80400000);
    pub static ref APP_SIZE_LIMIT: usize = option_env!("USER_BASE_ADDRESS_STEP")
        .and_then(|s| usize::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x00020000);
}
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

#[link_section = ".bss"]
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, trap_cx: TrapContext, task_cx: TaskContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        let task_cx_ptr =
            (trap_cx_ptr as usize - core::mem::size_of::<TaskContext>()) as *mut TaskContext;
        unsafe {
            *task_cx_ptr = task_cx;
        }
        task_cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

fn init_stack(kstack: &mut KernelStack, ustack: &UserStack, pc: usize, user_satp: usize) -> usize {
    kstack.push_context(
        TrapContext::new(pc, ustack.get_sp(), user_satp, kstack.get_sp()),
        TaskContext::goto_launch(),
    )
}

fn app_base_address(id: usize) -> usize {
    (*APP_BASE_ADDRESS) + id * (*APP_SIZE_LIMIT)
}

pub struct AppManager {
    pub app_num: usize,
    app_span: [(usize, usize); MAX_APP_NUM],
}

unsafe impl Sync for AppManager {}

lazy_static! {
    pub static ref APP_MANAGER: AppManager = {
        extern "C" {
            fn app_list();
        }
        let app_list = app_list as usize as *const usize;
        let app_num = unsafe { app_list.read_volatile() };
        if app_num > MAX_APP_NUM {
            panic!("Too many apps!");
        }
        let mut app_span = [(0, 0); MAX_APP_NUM];
        for (i, span) in app_span.iter_mut().enumerate().take(app_num) {
            *span = unsafe {
                (
                    app_list.add(1 + 2 * i).read_volatile(),
                    app_list.add(2 + 2 * i).read_volatile(),
                )
            }
        }
        AppManager { app_num, app_span }
    };
}

impl AppManager {
    pub fn print_info(&self) {
        println!("[kernel] app_num: {}", APP_MANAGER.app_num);
        for i in 0..APP_MANAGER.app_num {
            println!(
                "[kernel]     {}: {:#x?}-{:#x?}",
                i, APP_MANAGER.app_span[i].0, APP_MANAGER.app_span[i].1
            );
        }
        println!("[kernel] APP_BASE_ADDRESS: {:#x?}", *APP_BASE_ADDRESS);
        println!("[kernel] APP_SIZE_LIMIT: {:#x?}", *APP_SIZE_LIMIT);
    }
    pub fn load_app(&self, id: usize) -> LoadedApp {
        let (app_start_address, app_end_address) = self.app_span[id];
        let app_bin = unsafe {
            slice::from_raw_parts(
                app_start_address as *const u8,
                app_end_address - app_start_address,
            )
        };
        let sapp = app_base_address(id);
        let eapp = app_base_address(id) + (*APP_SIZE_LIMIT);
        println!("[kernel] base_addr: {:#x?}", sapp);
        let app_dest = unsafe { slice::from_raw_parts_mut(sapp as *mut u8, *APP_SIZE_LIMIT) };
        app_dest.fill(0);
        app_dest
            .get_mut(0..app_bin.len())
            .expect("App binary is too big!")
            .copy_from_slice(app_bin);
        unsafe {
            llvm_asm!("fence.i" :::: "volatile");
        }
        let aspace = AddressSpace::new();
        // map user app
        aspace.map(
            VMObjectPhysical::from_range(PhysAddr::from(sapp), PhysAddr::from(eapp)),
            0,
            VirtAddr::from(sapp).floor_page_num(),
            None,
            PTEFlags::RWX | PTEFlags::U,
        );
        // map user stack
        let sstack = USER_STACK[id].data.as_ptr() as usize;
        let estack = sstack + USER_STACK_SIZE;
        aspace.map(
            VMObjectPhysical::from_range(PhysAddr::from(sstack), PhysAddr::from(estack)),
            0,
            VirtAddr::from(sstack).floor_page_num(),
            None,
            PTEFlags::R | PTEFlags::W | PTEFlags::U,
        );
        // map trampoline
        aspace.map(
            TRAMPOLINE.clone(),
            0,
            *TRAMPOLINE_BASE_VPN,
            None,
            PTEFlags::R | PTEFlags::X,
        );
        // map kernel stack
        let kstack_vmo = VMObjectPaged::new(page_count(KERNEL_STACK_SIZE));
        aspace.map(
            kstack_vmo.clone(),
            0,
            *KSTACK_BASE_VPN,
            None,
            PTEFlags::R | PTEFlags::W,
        );
        let vskstack =
            usize::from(TRAMPOLINE_BASE_VPN.addr()) - (KERNEL_STACK_SIZE + PAGE_SIZE) * (id + 1);
        KERNEL_ASPACE.map(
            kstack_vmo,
            0,
            VirtAddr::from(vskstack).floor_page_num(),
            None,
            PTEFlags::R | PTEFlags::W,
        );
        let kstack = unsafe { (vskstack as *mut KernelStack).as_mut().unwrap() };
        let token = aspace.token();
        LoadedApp {
            aspace,
            kernel_sp: init_stack(kstack, &USER_STACK[id], app_base_address(id), token),
        }
    }
}

pub struct LoadedApp {
    pub aspace: Arc<AddressSpace>,
    pub kernel_sp: usize,
}
