#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]
#![feature(slice_fill)]
#![feature(const_in_array_repeat_expressions)]
#![feature(alloc_error_handler)]

global_asm!(include_str!("entry.asm"));

extern crate alloc;

#[macro_use]
mod console;
mod backtrace;
mod heap;
mod lang;
mod loader;
mod mm;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use loader::APP_MANAGER;
use task::TASK_MANAGER;

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    heap::init();
    backtrace::init();
    mm::init();
    trap::init();
    timer::init();
    timer::schedule_next();
    APP_MANAGER.print_info();
    TASK_MANAGER.launch_first_task();
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
