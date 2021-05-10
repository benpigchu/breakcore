#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use core::cmp::{Ord, Ordering};
use user::sys_fork;

#[no_mangle]
fn main() -> i32 {
    let result = sys_fork();
    match result.cmp(&0) {
        Ordering::Less => {
            println!("Fork failed!")
        }
        Ordering::Equal => {
            println!("Fork success, this is child")
        }
        Ordering::Greater => {
            println!("Fork success into pid {:?}", result)
        }
    }
    0
}
