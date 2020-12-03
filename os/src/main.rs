#![no_std]
#![no_main]
#![feature(global_asm)]

global_asm!(include_str!("entry.asm"));

mod lang;

#[no_mangle]
pub fn rust_main() -> ! {
    loop {}
}