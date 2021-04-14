mod fs;
mod misc;
mod mm;
mod proc;

use fs::*;
use log::*;
use misc::*;
use mm::*;
use proc::*;

const SYSCALL_EXIT: usize = 93;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;

pub fn syscall(syscall_id: usize, args0: usize, args1: usize, args2: usize) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args0, args1.into(), args2),
        SYSCALL_EXIT => sys_exit(args0 as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args0.into()),
        SYSCALL_SET_PRIORITY => sys_set_priority(args0 as isize),
        SYSCALL_MMAP => sys_mmap(args0.into(), args1, args2),
        SYSCALL_MUNMAP => sys_munmap(args0.into(), args1),
        _ => {
            warn!("Unsupported syscall_id: {}", syscall_id);
            sys_exit(-1)
        }
    }
}
