#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::sys_yield;

const WIDTH: usize = 10;
const HEIGHT: usize = 5;

#[no_mangle]
fn main() -> i32 {
    for i in 0..HEIGHT {
        for _ in 0..WIDTH {
            print!("A");
        }
        println!(" [{}/{}]", i + 1, HEIGHT);
        sys_yield();
    }
    println!("Test write_a OK!");
    0
}
