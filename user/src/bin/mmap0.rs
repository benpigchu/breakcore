#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::{sys_mmap, MMapprot};

#[no_mangle]
fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 0x1000;
    let prot = MMapprot::READ | MMapprot::WRITE;
    assert_eq!(len as isize, sys_mmap(start, len, prot));
    println!("MMap test 0 mapped a page");
    for i in start..(start + len) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            *addr = i as u8;
        }
    }
    for i in start..(start + len) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            assert_eq!(*addr, i as u8);
        }
    }
    println!("MMap test 0 OK!");
    0
}
