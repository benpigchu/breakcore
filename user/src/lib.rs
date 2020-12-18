#![no_std]
#![feature(linkage)]
#![feature(llvm_asm)]

#[macro_use]
pub mod console;
mod lang;
mod syscall;

use syscall::*;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    let exit_code = main();
    sys_exit(exit_code)
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    for addr in (start_bss as usize)..(end_bss as usize) {
        unsafe { (addr as *mut u8).write_volatile(0) }
    }
}
