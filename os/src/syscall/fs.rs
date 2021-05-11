use crate::mm::addr::*;
use crate::task::TASK_MANAGER;
use alloc::vec;

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, addr: VirtAddr, len: usize) -> isize {
    let mut buffer = vec![0; len];
    let aspace = if let Some(aspace) = TASK_MANAGER.current_task().map(|task| task.aspace()) {
        aspace
    } else {
        return -1;
    };
    let real_len = aspace.read(addr, &mut buffer, true);
    let str = unsafe { core::str::from_utf8_unchecked(&buffer) };
    match fd {
        FD_STDOUT => {
            print!("{}", str);
            real_len as isize
        }
        _ => -1,
    }
}
