use core::isize;

use super::{MMapprot, TimeVal};

pub const STDOUT: usize = 1;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;

fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let mut ret: isize;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (id)
            : "memory"
            : "volatile"
        );
    }
    ret
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, fd, buffer.as_ptr() as usize, buffer.len())
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, exit_code as usize, 0, 0);
    unreachable!("We are already exitted!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, 0, 0, 0)
}

pub fn sys_get_time(time_val: &mut TimeVal) -> isize {
    syscall(SYSCALL_GET_TIME, time_val as *mut _ as usize, 0, 0)
}

pub fn sys_set_priority(priority: isize) -> isize {
    syscall(SYSCALL_SET_PRIORITY, priority as usize, 0, 0)
}

pub fn sys_mmap(start: usize, len: usize, prot: MMapprot) -> isize {
    syscall(SYSCALL_MMAP, start, len, prot.bits())
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    syscall(SYSCALL_MUNMAP, start, len, 0)
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, 0, 0, 0)
}

pub fn sys_exec(name: &str) -> isize {
    let mut cstr = name.as_bytes().to_vec();
    cstr.push(0);
    syscall(SYSCALL_EXEC, cstr.as_ptr() as usize, 0, 0)
}
