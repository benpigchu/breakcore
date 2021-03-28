use crate::loader::*;
use crate::mm::aspace::AddressSpace;
use crate::sbi::shutdown;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

mod context;
pub use context::*;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

struct Task {
    kernel_sp: usize,
    status: TaskStatus,
    aspace: Option<Arc<AddressSpace>>,
}

impl Task {
    fn get_kernel_sp_ptr(&self) -> usize {
        &self.kernel_sp as *const usize as usize
    }
}

pub struct TaskManager {
    app_num: usize,
    inner: Mutex<TaskManagerInner>,
}
pub struct TaskManagerInner {
    current: usize,
    tasks: [Task; MAX_APP_NUM],
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager {
        app_num: APP_MANAGER.app_num,
        inner: Mutex::new(TaskManagerInner {
            current: 0,
            tasks: [Task {
                kernel_sp: 0,
                status: TaskStatus::UnInit,
                aspace: None
            }; MAX_APP_NUM]
        }),
    };
}

impl TaskManager {
    pub fn launch_first_task(&self) -> ! {
        let mut inner = self.inner.lock();
        inner.init_task(0);
        let next_kernel_sp_ptr = inner.tasks[0].get_kernel_sp_ptr();
        let current_kernel_sp = 0usize;
        let current_kernel_sp_ptr = &current_kernel_sp as *const usize as usize;
        inner.tasks[0].status = TaskStatus::Running;
        drop(inner);
        unsafe {
            __switch(current_kernel_sp_ptr, next_kernel_sp_ptr);
        }
        unreachable!("We will no use boot_stack from here!");
    }

    fn switch_to_task(&self, current: usize, next: usize) {
        if current == next {
            return;
        }
        let inner = self.inner.lock();
        let current_kernel_sp_ptr = inner.tasks[current].get_kernel_sp_ptr();
        let next_kernel_sp_ptr = inner.tasks[next].get_kernel_sp_ptr();
        drop(inner);
        unsafe {
            __switch(current_kernel_sp_ptr, next_kernel_sp_ptr);
        }
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.lock();
        for i in 0..self.app_num {
            let id = (i + inner.current + 1) % self.app_num;
            if matches!(
                inner.tasks[id].status,
                TaskStatus::UnInit | TaskStatus::Ready | TaskStatus::Running
            ) {
                return Some(id);
            }
        }
        None
    }

    pub fn switch_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.lock();
            let current = inner.current;
            inner.current = next;
            if inner.tasks[current].status == TaskStatus::Running {
                inner.tasks[current].status = TaskStatus::Ready
            }
            if inner.tasks[next].status == TaskStatus::UnInit {
                inner.init_task(next);
            }
            inner.tasks[next].status = TaskStatus::Running;
            drop(inner);
            self.switch_to_task(current, next)
        } else {
            println!("[kernel] No more app!");
            shutdown()
        }
    }

    pub fn exit_task(&self, exit_code: i32) -> ! {
        let mut inner = self.inner.lock();
        let current = inner.current;
        println!(
            "[kernel] user program {} exited, code: {:#x?}",
            current, exit_code
        );
        inner.tasks[current].status = TaskStatus::Exited;
        drop(inner);
        self.switch_task();
        unreachable!("We should not switch back to exited task!");
    }

    pub fn current_aspace(&self) -> Option<Arc<AddressSpace>> {
        let inner = self.inner.lock();
        inner.tasks[inner.current]
            .aspace
            .as_ref()
            .map(|aspace| aspace.clone())
    }
}

impl TaskManagerInner {
    fn init_task(&mut self, id: usize) {
        println!("[kernel] load app: {}", id);
        let loaded_app = APP_MANAGER.load_app(id);
        self.tasks[id].kernel_sp = loaded_app.kernel_sp;
        self.tasks[id].aspace = Some(loaded_app.aspace)
    }
}
