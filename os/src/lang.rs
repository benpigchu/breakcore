use crate::backtrace::print_backtrace;
use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\u{1B}[31m{}\u{1B}[0m", info);
    print_backtrace();
    shutdown()
}
