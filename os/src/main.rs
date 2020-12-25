#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]
#![feature(slice_fill)]
#![feature(const_in_array_repeat_expressions)]

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("embed_app.asm"));

#[macro_use]
mod console;
mod backtrace;
mod task;
mod loader;
mod lang;
mod sbi;
mod syscall;
mod trap;

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    loader::init();
    loader::run_next_app();
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
