#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]
#![feature(slice_fill)]

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("embed_app.asm"));

#[macro_use]
mod console;
mod backtrace;
mod batch;
mod lang;
mod logging;
mod sbi;
mod syscall;
mod trap;

use log::*;

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    info!("Hello, world!");
    trap::init();
    batch::init();
    batch::run_next_app();
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
}
