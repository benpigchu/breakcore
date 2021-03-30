use crate::mm::addr::*;
use crate::mm::aspace::{AddressSpace, KERNEL_ASPACE, KSTACK_BASE_VPN, TRAMPOLINE_BASE_VPN};
use crate::mm::vmo::{VMObject, VMObjectPaged, VMObjectPhysical, TRAMPOLINE};
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

fn init_stack(kstack: &mut KernelStack, ustack_sp: usize, pc: usize, user_satp: usize) -> usize {
    kstack.push_context(
        TrapContext::new(pc, ustack_sp, user_satp, kstack.get_sp()),
        TaskContext::goto_launch(),
    )
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
        let app_bin_data = unsafe {
            slice::from_raw_parts(
                app_start_address as *const u8,
                app_end_address - app_start_address,
            )
        };
        // TODO: load ELF
        let aspace = AddressSpace::new();
        let mut elf_vaddr_end = 0usize;
        let mut stack_pte_flags = PTEFlags::U;

        use object::elf::*;
        use object::read::elf::*;
        use object::{Bytes, LittleEndian};
        let app_bin_bytes = Bytes(app_bin_data);
        // we are using little endian elf64 format
        let file_header = FileHeader64::<LittleEndian>::parse(app_bin_bytes).unwrap();
        println!("[kernel] Parsing ELF...");
        assert!(file_header.is_little_endian());
        assert!(file_header.is_class_64());
        assert_eq!(file_header.e_machine(LittleEndian), EM_RISCV);
        assert_eq!(file_header.e_type(LittleEndian), ET_EXEC);
        let entry = file_header.e_entry(LittleEndian) as usize;
        let program_headers = file_header
            .program_headers(LittleEndian, app_bin_bytes)
            .unwrap();
        for program_header in program_headers {
            fn pte_flags_from_ph_flags(ph_flags: u32) -> PTEFlags {
                let mut pte_flags = PTEFlags::empty();
                if ph_flags & PF_R != 0 {
                    pte_flags.insert(PTEFlags::R)
                }
                if ph_flags & PF_W != 0 {
                    pte_flags.insert(PTEFlags::W)
                }
                if ph_flags & PF_X != 0 {
                    pte_flags.insert(PTEFlags::X)
                }
                pte_flags
            }
            match program_header.p_type(LittleEndian) {
                PT_LOAD => {
                    println!("[kernel]     ELF segment:LOAD");
                    let ph_flags = program_header.p_flags(LittleEndian);
                    let mut pte_flags = pte_flags_from_ph_flags(ph_flags);
                    println!("[kernel]         flags:{:?}", pte_flags);
                    pte_flags.insert(PTEFlags::U);
                    let vaddr_start = program_header.p_vaddr(LittleEndian) as usize;
                    let mem_size = program_header.p_memsz(LittleEndian) as usize;
                    let vaddr_end = vaddr_start + mem_size;
                    println!(
                        "[kernel]         vaddr:{:#x?}-{:#x?}",
                        vaddr_start, vaddr_end
                    );
                    if vaddr_start % PAGE_SIZE != 0 {
                        panic!("ELF LOAD segment start address not page aligned")
                    }
                    let vmo = VMObjectPaged::new(page_count(mem_size));
                    let wrote_size = vmo.write(
                        0,
                        program_header
                            .data_as_array(LittleEndian, app_bin_bytes)
                            .unwrap(),
                    );
                    assert_eq!(mem_size, wrote_size);
                    aspace
                        .map(
                            vmo,
                            0,
                            VirtAddr::from(vaddr_start).floor_page_num(),
                            None,
                            pte_flags,
                        )
                        .unwrap();
                    elf_vaddr_end = usize::max(elf_vaddr_end, vaddr_end)
                }
                PT_GNU_STACK => {
                    println!("[kernel]     ELF segment:STACK");
                    let ph_flags = program_header.p_flags(LittleEndian);
                    let pte_flags = pte_flags_from_ph_flags(ph_flags);
                    println!("[kernel]         flags:{:?}", pte_flags);
                    stack_pte_flags.insert(pte_flags)
                }
                PT_INTERP => {
                    panic!("Dynamic linking is not supported");
                }
                other => {
                    println!("[kernel]     ELF segment:{:?}", other);
                }
            }
        }
        unsafe {
            llvm_asm!("fence.i" :::: "volatile");
        }
        // map user stack
        let sstack = USER_STACK[id].data.as_ptr() as usize;
        let estack = sstack + USER_STACK_SIZE;
        let vsstack_pn = VirtAddr::from(elf_vaddr_end + PAGE_SIZE).ceil_page_num();
        aspace.map(
            VMObjectPhysical::from_range(PhysAddr::from(sstack), PhysAddr::from(estack)),
            0,
            vsstack_pn,
            None,
            stack_pte_flags,
        );
        println!("[kernel] map user stack at {:#x?}", vsstack_pn.addr());
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
            kernel_sp: init_stack(
                kstack,
                usize::from(vsstack_pn.addr()) + USER_STACK_SIZE,
                entry,
                token,
            ),
        }
    }
}

pub struct LoadedApp {
    pub aspace: Arc<AddressSpace>,
    pub kernel_sp: usize,
}
