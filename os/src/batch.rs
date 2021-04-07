use crate::{sbi::shutdown, trap::context::TrapContext};
use core::{cell::RefCell, slice};
use lazy_static::*;
use log::*;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80080000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> usize {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

fn load_app(id: usize) {
    let (app_start_address, app_end_address) = APP_MANAGER.app_span[id];
    let app_bin = unsafe {
        slice::from_raw_parts(
            app_start_address as *const u8,
            app_end_address - app_start_address,
        )
    };
    let app_dest =
        unsafe { slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT) };
    app_dest.fill(0);
    app_dest
        .get_mut(0..app_bin.len())
        .expect("App binary is too big!")
        .copy_from_slice(app_bin);
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
    }
}

fn launch_app() -> ! {
    extern "C" {
        fn __restore(kernel_sp: usize);
    }
    unsafe {
        __restore(
            KERNEL_STACK.push_context(TrapContext::new(APP_BASE_ADDRESS, USER_STACK.get_sp())),
        );
    }
    unreachable!("We are already in user space!");
}

pub fn run_next_app() -> ! {
    let mut next_app = APP_MANAGER.next_app_id.borrow_mut();
    if *next_app >= APP_MANAGER.app_num {
        info!("No more app!");
        shutdown()
    } else {
        info!("load app: {}", *next_app);
        load_app(*next_app);
        *next_app += 1;
        drop(next_app);
        launch_app()
    }
}

pub fn exit_app() -> ! {
    run_next_app()
}

struct AppManager {
    app_num: usize,
    app_span: [(usize, usize); MAX_APP_NUM],
    next_app_id: RefCell<usize>,
}

unsafe impl Sync for AppManager {}

lazy_static! {
    static ref APP_MANAGER: AppManager = {
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
        AppManager {
            app_num,
            app_span,
            next_app_id: RefCell::new(0),
        }
    };
}

pub fn init() {
    initialize(&APP_MANAGER);
    info!("app_num: {}", APP_MANAGER.app_num);
    for i in 0..APP_MANAGER.app_num {
        info!(
            "    {}: {:#x?}-{:#x?}",
            i, APP_MANAGER.app_span[i].0, APP_MANAGER.app_span[i].1
        );
    }
}
