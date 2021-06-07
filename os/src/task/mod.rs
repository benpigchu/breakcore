use crate::loader::*;
use crate::mm::aspace::{create_user_aspace, AddressSpace};
use crate::sbi::shutdown;
use crate::trap::context::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
use log::*;
use spin::Mutex;

mod context;
pub use context::*;
mod sched;
use sched::{create_scheduler, Scheduler, SchedulerImpl};
pub mod pid;
use pid::PidHandle;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

struct TaskInner<SD: Default> {
    pid: PidHandle,
    kernel_sp: usize,
    trap_cx_ptr: usize,
    status: TaskStatus,
    aspace: Arc<AddressSpace>,
    sched_data: SD,
    priority: usize,
}

pub struct Task<SD: Default> {
    inner: Mutex<TaskInner<SD>>,
}

impl<SD: Default> TaskInner<SD> {
    fn get_kernel_sp_ptr(&self) -> usize {
        &self.kernel_sp as *const usize as usize
    }
}

impl<SD: Default> Task<SD> {
    fn new_base() -> Arc<Self> {
        let pid = PidHandle::alloc();
        let kstack = pid.kernel_stack();

        let (aspace, trap_cx_ptr) = create_user_aspace();

        let token = aspace.token();
        let trap_cx_ref = unsafe { (trap_cx_ptr as *mut TrapContext).as_mut() }.unwrap();
        let user_cx = TrapContext::new(token, kstack.get_bottom_sp(), trap_cx_ptr);
        *trap_cx_ref = user_cx;

        Arc::new(Self {
            inner: Mutex::new(TaskInner::<SD> {
                pid,
                kernel_sp: kstack.get_init_sp(),
                trap_cx_ptr,
                status: TaskStatus::Ready,
                aspace,
                sched_data: Default::default(),
                priority: 2,
            }),
        })
    }

    fn new_init(app_id: usize) -> Arc<Self> {
        info!("task from app: {}", app_id);
        let task = Self::new_base();
        let inner = task.inner.lock();
        let trap_cx_ref = unsafe { (inner.trap_cx_ptr as *mut TrapContext).as_mut() }.unwrap();

        let loaded_elf = APP_MANAGER
            .load_elf(app_id, &inner.aspace)
            .expect("Init ELF load failed");
        trap_cx_ref.set_pc(loaded_elf.entry);
        trap_cx_ref.set_sp(loaded_elf.user_sp);

        drop(inner);
        task
    }

    pub fn new_fork(&self) -> Option<Arc<Self>> {
        let inner = self.inner.lock();
        info!("task fork from pid: {}", inner.pid.value());
        let trap_cx_ref = unsafe { (inner.trap_cx_ptr as *mut TrapContext).as_mut() }.unwrap();

        let task = Self::new_base();
        let task_inner = task.inner.lock();
        let task_trap_cx_ref =
            unsafe { (task_inner.trap_cx_ptr as *mut TrapContext).as_mut() }.unwrap();

        task_inner.aspace.fork_from(&inner.aspace)?;
        task_trap_cx_ref.x = trap_cx_ref.x;
        task_trap_cx_ref.sepc = trap_cx_ref.sepc;
        task_trap_cx_ref.sstatus = trap_cx_ref.sstatus;
        // fork syscall returns 0 for new task
        trap_cx_ref.x[10] = 0;
        drop(task_inner);
        Some(task)
    }

    pub fn aspace(&self) -> Arc<AddressSpace> {
        self.inner.lock().aspace.clone()
    }

    pub fn set_priority(&self, priority: usize) {
        self.inner.lock().priority = priority
    }

    pub fn trap_cx_ptr(&self) -> usize {
        self.inner.lock().trap_cx_ptr
    }

    pub fn pid(&self) -> usize {
        self.inner.lock().pid.value()
    }
}

type TaskImpl = Task<<SchedulerImpl as Scheduler>::Data>;

pub struct TaskManager {
    app_num: usize,
    inner: Mutex<TaskManagerInner>,
}
pub struct TaskManagerInner {
    current: Option<Arc<TaskImpl>>,
    last: Option<Arc<TaskImpl>>,
    scheduler: SchedulerImpl,
    ready_tasks: Vec<Arc<TaskImpl>>,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager {
        app_num: APP_MANAGER.app_num,
        inner: Mutex::new(TaskManagerInner {
            current: None,
            last: None,
            scheduler: create_scheduler(),
            ready_tasks: Vec::new()
        }),
    };
}

impl TaskManagerInner {
    fn take_next(&mut self) -> Option<Arc<TaskImpl>> {
        let index = self.scheduler.pick_next(&self.ready_tasks);
        if let Some(task_index) = index {
            Some(self.ready_tasks.remove(task_index))
        } else {
            None
        }
    }
}

impl TaskManager {
    pub fn launch(&self) -> ! {
        let mut inner = self.inner.lock();
        for id in 0..self.app_num {
            inner.ready_tasks.push(Task::new_init(id))
        }
        drop(inner);
        self.switch_task();
        unreachable!("We will no use boot_stack from here!");
    }

    pub fn switch_task(&self) {
        let mut inner = self.inner.lock();
        let current = inner.current.take();
        let current_kernel_sp_ptr: usize;
        if let Some(current_task) = current.as_ref() {
            inner.scheduler.proc_tick(&current_task);
            let mut current_inner = current_task.inner.lock();
            if current_inner.status == TaskStatus::Running {
                current_inner.status = TaskStatus::Ready;
                inner.ready_tasks.push(current_task.clone())
            }
            current_kernel_sp_ptr = current_inner.get_kernel_sp_ptr();
            drop(current_inner);
        } else {
            let current_kernel_sp = 0usize;
            current_kernel_sp_ptr = &current_kernel_sp as *const usize as usize;
        }
        inner.last = current;
        if let Some(next_task) = inner.take_next() {
            let mut next_inner = next_task.inner.lock();
            next_inner.status = TaskStatus::Running;
            let next_kernel_sp_ptr = next_inner.get_kernel_sp_ptr();
            drop(next_inner);
            inner.current = Some(next_task);
            drop(inner);
            if current_kernel_sp_ptr != next_kernel_sp_ptr {
                unsafe {
                    __switch(current_kernel_sp_ptr, next_kernel_sp_ptr);
                }
            }
            self.inner.lock().last.take();
        } else {
            drop(inner);
            info!("No more app!");
            shutdown()
        }
    }

    pub fn exit_task(&self, exit_code: i32) -> ! {
        let inner = self.inner.lock();
        let current = inner.current.as_ref().unwrap();
        let mut current_inner = current.inner.lock();
        info!(
            "user program {} exited, code: {:#x?}",
            current_inner.pid.value(),
            exit_code
        );
        current_inner.status = TaskStatus::Exited;
        drop(current_inner);
        drop(inner);
        self.switch_task();
        unreachable!("We should not switch back to exited task!");
    }

    pub fn current_task(&self) -> Option<Arc<TaskImpl>> {
        self.inner.lock().current.clone()
    }

    pub fn add_task(&self, task: Arc<TaskImpl>) {
        self.inner.lock().ready_tasks.push(task)
    }
}
