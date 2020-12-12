use crate::backtrace::print_backtrace;
use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    print_backtrace();
    shutdown()
}
