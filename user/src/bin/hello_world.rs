#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

#[no_mangle]
fn main() -> i32 {
    println!("Hello, world!");
    0
}
