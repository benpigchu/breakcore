use crate::task::TASK_MANAGER;

pub fn sys_exit(exit_code: i32) -> ! {
    TASK_MANAGER.exit_task(exit_code);
}

pub fn sys_yield() -> isize {
    TASK_MANAGER.switch_task();
    0
}
