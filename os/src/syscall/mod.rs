mod proc;

use proc::*;

const SYSCALL_EXIT: usize = 93;

pub fn syscall(syscall_id: usize, args0: usize, _args1: usize, _args2: usize) -> isize {
    match syscall_id {
        SYSCALL_EXIT => sys_exit(args0 as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
