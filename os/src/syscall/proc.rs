use crate::batch::exit_app;

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] user program exited, code: {:#x?}", exit_code);
    exit_app();
}
