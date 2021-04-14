use crate::task::TASK_MANAGER;

pub fn sys_exit(exit_code: i32) -> ! {
    TASK_MANAGER.exit_task(exit_code);
}

pub fn sys_yield() -> isize {
    TASK_MANAGER.switch_task();
    0
}

pub fn sys_set_priority(priority: isize) -> isize {
    if priority < 2 {
        return -1;
    }
    TASK_MANAGER.set_current_task_priority(priority as usize);
    priority
}
