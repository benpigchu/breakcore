use crate::mm::addr::*;
use crate::task::TASK_MANAGER;
use crate::timer::{get_time_ms, MSEC_PER_SEC, USEC_PER_MSEC};
use core::mem::size_of;
use core::slice;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(time_val_addr: VirtAddr) -> isize {
    let aspace = if let Some(aspace) = TASK_MANAGER.current_aspace() {
        aspace
    } else {
        return -1;
    };
    let time_ms = get_time_ms();
    let time_val = TimeVal {
        sec: time_ms / MSEC_PER_SEC,
        usec: USEC_PER_MSEC * (time_ms % MSEC_PER_SEC),
    };
    let buf =
        unsafe { slice::from_raw_parts(&time_val as *const _ as *const u8, size_of::<TimeVal>()) };
    if aspace.write(time_val_addr, buf, true) == buf.len() {
        0
    } else {
        -1
    }
}
