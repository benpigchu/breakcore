use crate::{task::TaskContext, trap::context::TrapContext};
use core::slice;
use lazy_static::*;
use log::*;

global_asm!(include_str!("embed_app.asm"));

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 16;
lazy_static! {
    static ref APP_BASE_ADDRESS: usize = option_env!("USER_BASE_ADDRESS_START")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x80100000);
    static ref APP_SIZE_LIMIT: usize = option_env!("USER_BASE_ADDRESS_STEP")
        .and_then(|s| s.parse().ok())
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

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];
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

pub fn init_stack(id: usize) -> usize {
    KERNEL_STACK[id].push_context(
        TrapContext::new(app_base_address(id), USER_STACK[id].get_sp()),
        TaskContext::goto_restore(),
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
        info!("app_num: {}", APP_MANAGER.app_num);
        for i in 0..APP_MANAGER.app_num {
            info!(
                "    {}: {:#x?}-{:#x?}",
                i, APP_MANAGER.app_span[i].0, APP_MANAGER.app_span[i].1
            );
        }
        info!("APP_BASE_ADDRESS: {:#x?}", *APP_BASE_ADDRESS);
        info!("APP_SIZE_LIMIT: {:#x?}", *APP_SIZE_LIMIT);
    }
    pub fn load_app(&self, id: usize) {
        let (app_start_address, app_end_address) = self.app_span[id];
        let app_bin = unsafe {
            slice::from_raw_parts(
                app_start_address as *const u8,
                app_end_address - app_start_address,
            )
        };
        info!("base_addr: {:#x?}", app_base_address(id));
        let app_dest =
            unsafe { slice::from_raw_parts_mut(app_base_address(id) as *mut u8, *APP_SIZE_LIMIT) };
        app_dest.fill(0);
        app_dest
            .get_mut(0..app_bin.len())
            .expect("App binary is too big!")
            .copy_from_slice(app_bin);
        unsafe {
            llvm_asm!("fence.i" :::: "volatile");
        }
    }
}
