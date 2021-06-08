use crate::mm::addr::*;
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
    TASK_MANAGER
        .current_task()
        .unwrap()
        .set_priority(priority as usize);
    priority
}

pub fn sys_fork() -> isize {
    if let Some(fork) = TASK_MANAGER
        .current_task()
        .and_then(|current| current.new_fork())
    {
        let pid = fork.pid();
        TASK_MANAGER.add_task(fork);
        return pid as isize;
    }
    -1
}

pub fn sys_exec(name: VirtAddr) -> isize {
    let task = TASK_MANAGER.current_task().unwrap();
    let app_name = task.aspace().read_cstr(name, true);
    if task.exec(&app_name).is_some() {
        0
    } else {
        -1
    }
}
