use crate::task::TASK_MANAGER;

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] user program exited, code: {:#x?}", exit_code);
    TASK_MANAGER.exit_app();
}
