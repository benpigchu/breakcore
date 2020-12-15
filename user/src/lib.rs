#![no_std]
#![feature(linkage)]

mod lang;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    main();
    panic!("TODO: exit syscall");
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
