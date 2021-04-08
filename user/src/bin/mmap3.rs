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
    println!("MMap test 3 mapped a page");
    println!("MMap test 3 testing illegal mmap call..");
    assert_eq!(sys_mmap(start - len, len + 1, prot), -1);
    assert_eq!(sys_mmap(start + len + 1, len, prot), -1);
    assert_eq!(sys_mmap(start + len, len, MMapprot::empty()), -1);
    assert_eq!(
        sys_mmap(start + len, len, unsafe {
            MMapprot::from_bits_unchecked(0b1011)
        }),
        -1
    );
    println!("MMap test 3 OK!");
    0
}
