use crate::syscall::*;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sys_exit(i32::MIN)
}
