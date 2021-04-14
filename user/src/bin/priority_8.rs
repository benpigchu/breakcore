#![no_std]
#![no_main]

#[macro_use]
extern crate breakcore_user as user;

use user::{sys_get_time, sys_set_priority, TimeVal};

fn get_time() -> isize {
    let mut time_val = TimeVal::default();
    match sys_get_time(&mut time_val) {
        0 => ((time_val.sec & 0xffff) * 1000 + time_val.usec / 1000) as isize,
        _ => -1,
    }
}

const MAX_TIME: isize = 1000;
fn count_during(prio: isize) -> isize {
    let start_time = get_time();
    let mut acc = 0;
    sys_set_priority(prio);
    loop {
        spin_delay();
        acc += 1;
        if acc % 400 == 0 {
            let time = get_time() - start_time;
            if time > MAX_TIME {
                return acc;
            }
        }
    }
}

fn spin_delay() {
    let mut j = true;
    for _ in 0..10 {
        j = !j;
    }
}

#[no_mangle]
fn main() -> i32 {
    let prio = 8;
    let count = count_during(prio);
    println!("priority = {}, count = {}", prio, count);
    0
}
