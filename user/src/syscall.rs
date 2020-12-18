const SYSCALL_EXIT: usize = 93;

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

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, exit_code as usize, 0, 0);
    unreachable!("We are already exitted!");
}
