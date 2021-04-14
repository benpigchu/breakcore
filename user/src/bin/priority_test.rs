#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::sys_set_priority;

#[no_mangle]
fn main() -> i32 {
    assert_eq!(sys_set_priority(10), 10);
    assert_eq!(sys_set_priority(isize::MAX), isize::MAX);
    assert_eq!(sys_set_priority(0), -1);
    assert_eq!(sys_set_priority(1), -1);
    assert_eq!(sys_set_priority(-10), -1);
    println!("Test set_priority OK!");
    0
}
