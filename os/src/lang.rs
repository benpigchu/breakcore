use crate::backtrace::print_backtrace;
use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[kernel] {}", info);
    print_backtrace();
    shutdown()
}
