#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::{sys_mmap, MMapprot};

#[no_mangle]
fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 0x1000;
    let prot = MMapprot::READ;
    assert_eq!(len as isize, sys_mmap(start, len, prot));
    println!("MMap test 1 mapped a page");
    let addr: *mut u8 = start as *mut u8;
    println!("MMap test 1 try to trigger a write page fault...");
    unsafe {
        *addr = start as u8;
    }
    println!("MMap test 1 fail! Should cause error!");
    0
}
