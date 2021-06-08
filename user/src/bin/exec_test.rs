#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::sys_exec;

#[no_mangle]
fn main() -> i32 {
    println!("Exec test will execute non_exist, but it should fail");
    assert!(sys_exec("non_exist") < 0);
    println!("Exec test will execute hello_world");
    sys_exec("hello_world");
    0
}
