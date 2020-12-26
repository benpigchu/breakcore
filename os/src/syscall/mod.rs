mod fs;
mod proc;

use fs::*;
use proc::*;

const SYSCALL_EXIT: usize = 93;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_YIELD: usize = 124;

pub fn syscall(syscall_id: usize, args0: usize, args1: usize, args2: usize) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args0, args1 as *const u8, args2),
        SYSCALL_EXIT => sys_exit(args0 as i32),
        SYSCALL_YIELD => sys_yield(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
