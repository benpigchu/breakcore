use crate::timer::{get_time_ms, MSEC_PER_SEC, USEC_PER_MSEC};
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(time_val_ptr: usize) -> isize {
    let time_ms = get_time_ms();
    let time_val = TimeVal {
        sec: time_ms / MSEC_PER_SEC,
        usec: USEC_PER_MSEC * (time_ms % MSEC_PER_SEC),
    };
    unsafe { (time_val_ptr as *mut TimeVal).write_volatile(time_val) };
    0
}
