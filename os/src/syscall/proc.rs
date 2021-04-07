use crate::batch::exit_app;
use log::*;

pub fn sys_exit(exit_code: i32) -> ! {
    info!("user program exited, code: {:#x?}", exit_code);
    exit_app();
}
