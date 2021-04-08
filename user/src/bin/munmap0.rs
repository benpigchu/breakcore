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
    println!("MUnmap test 0 mapped page 1");
    assert_eq!(sys_mmap(start + len, len * 2, prot), (len * 2) as isize);
    println!("MUnmap test 0 mapped page 2&3");
    assert_eq!(sys_munmap(start, len), len as isize);
    println!("MUnmap test 0 unmapped page 1");
    assert_eq!(sys_mmap(start - len, len + 1, prot), (len * 2) as isize);
    println!("MUnmap test 0 mapped page 0&1");
    for i in (start - len)..(start + len * 3) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            *addr = i as u8;
        }
    }
    for i in (start - len)..(start + len * 3) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            assert_eq!(*addr, i as u8);
        }
    }
    println!("MUnmap test 0 OK!");
    0
}
