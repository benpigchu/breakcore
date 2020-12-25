#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]
#![feature(slice_fill)]
#![feature(const_in_array_repeat_expressions)]

global_asm!(include_str!("entry.asm"));

#[macro_use]
mod console;
mod backtrace;
mod lang;
mod loader;
mod sbi;
mod syscall;
mod task;
mod trap;

use loader::APP_MANAGER;
use task::TASK_MANAGER;

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    APP_MANAGER.print_info();
    TASK_MANAGER.run_next_app();
}

#[allow(dead_code)]
fn test_panic(depth: usize) -> ! {
    if depth > 0 {
        test_panic(depth - 1)
    } else {
        panic!("test panic");
    }
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    for addr in (sbss as usize)..(ebss as usize) {
        unsafe { (addr as *mut u8).write_volatile(0) }
    }
    println!("[kernel] bss: {:#x?}-{:#x?}", sbss as usize, ebss as usize);
}
