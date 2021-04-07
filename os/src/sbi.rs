#![allow(unused)]
use log::*;

const SBI_SET_TIMER: usize = 0x0;
const SBI_CONSOLE_PUTCHAR: usize = 0x1;
const SBI_CONSOLE_GETCHAR: usize = 0x2;
const SBI_CLEAR_IPI: usize = 0x3;
const SBI_SEND_IPI: usize = 0x4;
const SBI_REMOTE_FENCE_I: usize = 0x5;
const SBI_REMOTE_SFENCE_VMA: usize = 0x6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 0x7;
const SBI_SHUTDOWN: usize = 0x8;

#[inline(always)]
fn sbi_call(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
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

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    error!("It should shutdown!");
    #[allow(clippy::empty_loop)]
    loop {}
}

pub fn set_timer(timer: usize) {
    sbi_call(SBI_SET_TIMER, timer, 0, 0);
}
