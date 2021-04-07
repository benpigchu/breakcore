#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::{sys_get_time, sys_yield, TimeVal};

fn get_time() -> isize {
    let mut time_val = TimeVal::default();
    match sys_get_time(&mut time_val) {
        0 => ((time_val.sec & 0xffff) * 1000 + time_val.usec / 1000) as isize,
        _ => -1,
    }
}

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 3000;
    while get_time() < wait_for {
        sys_yield();
    }
    println!("Test sleep OK!");
    0
}
