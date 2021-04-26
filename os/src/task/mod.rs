use crate::loader::*;
use crate::mm::aspace::AddressSpace;
use crate::sbi::shutdown;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
use log::*;
use spin::Mutex;

mod context;
pub use context::*;
mod sched;
use sched::{create_scheduler, Scheduler, SchedulerImpl};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

struct TaskInner<SD: Default> {
    kernel_sp: usize,
    trap_cx_ptr: usize,
    status: TaskStatus,
    aspace: Option<Arc<AddressSpace>>,
    sched_data: SD,
    priority: usize,
}

struct Task<SD: Default> {
    inner: Mutex<TaskInner<SD>>,
}

impl<SD: Default> TaskInner<SD> {
    fn get_kernel_sp_ptr(&self) -> usize {
        &self.kernel_sp as *const usize as usize
    }
}

impl<SD: Default> Task<SD> {
    fn new(app_id: usize) -> Arc<Self> {
        info!("task from app: {}", app_id);
        let loaded_app = APP_MANAGER.load_app(app_id);
        Arc::new(Self {
            inner: Mutex::new(TaskInner::<SD> {
                kernel_sp: loaded_app.kernel_sp,
                trap_cx_ptr: loaded_app.trap_cx_ptr,
                status: TaskStatus::Ready,
                aspace: Some(loaded_app.aspace),
                sched_data: Default::default(),
                priority: 2,
            }),
        })
    }
}

pub struct TaskManager {
    app_num: usize,
    inner: Mutex<TaskManagerInner>,
}
pub struct TaskManagerInner {
    current: usize,
    scheduler: SchedulerImpl,
    tasks: Vec<Arc<Task<<SchedulerImpl as Scheduler>::Data>>>,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager {
        app_num: APP_MANAGER.app_num,
        inner: Mutex::new(TaskManagerInner {
            current: 0,
            scheduler: create_scheduler(),
            tasks: Vec::new()
        }),
    };
}

impl TaskManager {
    pub fn launch_first_task(&self) -> ! {
        let mut inner = self.inner.lock();
        for id in 0..self.app_num {
            inner.tasks.push(Task::new(id))
        }
        let task_id = inner
            .scheduler
            .pick_next(&inner.tasks[0..self.app_num])
            .unwrap();
        let mut task_inner = inner.tasks[task_id].inner.lock();
        let next_kernel_sp_ptr = task_inner.get_kernel_sp_ptr();
        let current_kernel_sp = 0usize;
        let current_kernel_sp_ptr = &current_kernel_sp as *const usize as usize;
        task_inner.status = TaskStatus::Running;
        drop(task_inner);
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
        let current_kernel_sp_ptr = inner.tasks[current].inner.lock().get_kernel_sp_ptr();
        let next_kernel_sp_ptr = inner.tasks[next].inner.lock().get_kernel_sp_ptr();
        drop(inner);
        unsafe {
            __switch(current_kernel_sp_ptr, next_kernel_sp_ptr);
        }
    }

    pub fn switch_task(&self) {
        let mut inner = self.inner.lock();
        let current = inner.current;
        inner.scheduler.proc_tick(&inner.tasks[current]);
        if let Some(next) = inner.scheduler.pick_next(&inner.tasks[0..self.app_num]) {
            inner.current = next;
            let mut current_inner = inner.tasks[current].inner.lock();
            if current_inner.status == TaskStatus::Running {
                current_inner.status = TaskStatus::Ready
            }
            drop(current_inner);
            let mut next_inner = inner.tasks[next].inner.lock();
            next_inner.status = TaskStatus::Running;
            drop(next_inner);
            drop(inner);
            self.switch_to_task(current, next)
        } else {
            drop(inner);
            info!("No more app!");
            shutdown()
        }
    }

    pub fn exit_task(&self, exit_code: i32) -> ! {
        let inner = self.inner.lock();
        let current = inner.current;
        info!("user program {} exited, code: {:#x?}", current, exit_code);
        inner.tasks[current].inner.lock().status = TaskStatus::Exited;
        drop(inner);
        self.switch_task();
        unreachable!("We should not switch back to exited task!");
    }

    pub fn current_aspace(&self) -> Option<Arc<AddressSpace>> {
        let inner = self.inner.lock();
        let aspace = inner.tasks[inner.current]
            .inner
            .lock()
            .aspace
            .as_ref()
            .cloned();
        aspace
    }

    pub fn set_current_task_priority(&self, priority: usize) {
        let inner = self.inner.lock();
        let current = inner.current;
        inner.tasks[current].inner.lock().priority = priority;
    }

    pub fn current_cx_ptr(&self) -> usize {
        let inner = self.inner.lock();
        let current = inner.current;
        let trap_cx_ptr = inner.tasks[current].inner.lock().trap_cx_ptr;
        trap_cx_ptr
    }
}
