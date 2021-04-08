#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::{sys_mmap, sys_munmap, MMapprot};

#[no_mangle]
fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 0x1000;
    let prot = MMapprot::READ | MMapprot::WRITE;
    assert_eq!(len as isize, sys_mmap(start, len, prot));
    println!("MUnmap test 1 mapped a page");
    println!("MUnmap test 1 testing illegal mmap call..");
    assert_eq!(sys_munmap(start, len + 1), -1);
    assert_eq!(sys_munmap(start + 1, len - 1), -1);
    println!("MUnmap test 1 OK!");
    0
}
