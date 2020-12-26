use crate::loader::*;
use crate::sbi::shutdown;
use core::cell::RefCell;
use lazy_static::*;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

struct Task {
    status: TaskStatus,
}

pub struct TaskManager {
    app_num: usize,
    inner: RefCell<TaskManagerInner>,
}
pub struct TaskManagerInner {
    current: usize,
    tasks: [Task; MAX_APP_NUM],
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager {
        app_num: APP_MANAGER.app_num,
        inner: RefCell::new(TaskManagerInner {
            current: 0,
            tasks: [Task {
                status: TaskStatus::UnInit
            }; MAX_APP_NUM]
        }),
    };
}

impl TaskManager {
    fn switch_to_task(&self, id: usize) -> ! {
        APP_MANAGER.load_app(id);
        extern "C" {
            fn __restore(kernel_sp: usize);
        }
        unsafe {
            __restore(init_stack(id));
        }
        unreachable!("We are already in user space!");
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        for i in 0..self.app_num {
            let id = (i + inner.current) % self.app_num;
            if matches!(
                inner.tasks[id].status,
                TaskStatus::UnInit | TaskStatus::Ready
            ) {
                return Some(id);
            }
        }
        None
    }

    pub fn switch_task(&self) -> ! {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current;
            inner.current = next;
            if inner.tasks[current].status == TaskStatus::Running {
                inner.tasks[current].status = TaskStatus::Ready
            }
            if inner.tasks[next].status == TaskStatus::UnInit {
                println!("[kernel] load app: {}", next);
                APP_MANAGER.load_app(next)
            }
            inner.tasks[next].status = TaskStatus::Running;
            drop(inner);
            self.switch_to_task(next)
        } else {
            println!("[kernel] No more app!");
            shutdown()
        }
    }

    pub fn exit_task(&self) -> ! {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current;
        inner.tasks[current].status = TaskStatus::Exited;
        drop(inner);
        self.switch_task()
    }
}
