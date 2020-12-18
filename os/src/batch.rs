use crate::trap::context::TrapContext;
use core::slice;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
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

pub fn load_app() {
    extern "C" {
        fn app_start();
        fn app_end();
    }
    let app_start_address = app_start as usize;
    let app_end_address = app_end as usize;
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

pub fn launch_app() -> ! {
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

pub fn exit_app() -> ! {
    loop {}
}
