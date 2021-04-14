use super::{Task, TaskStatus};
use core::cell::Cell;
pub trait Scheduler {
    type Data: Default + Copy;
    fn pick_next(&self, tasks: &[Task<Self::Data>]) -> Option<usize>;
    fn proc_tick(&self, tasks: Task<Self::Data>);
}

pub struct StrideScheduler {
    last: Cell<Option<usize>>,
}

#[derive(Default, Clone, Copy)]
pub struct StrideSchedulerData {
    stride: usize,
}

impl Scheduler for StrideScheduler {
    type Data = StrideSchedulerData;
    fn pick_next(&self, tasks: &[Task<Self::Data>]) -> Option<usize> {
        let base = self.last.get().map(|last| last + 1).unwrap_or(0);
        for i in 0..tasks.len() {
            let id = (i + base) % tasks.len();
            if matches!(
                tasks[id].status,
                TaskStatus::UnInit | TaskStatus::Ready | TaskStatus::Running
            ) {
                self.last.replace(Some(id));
                return Some(id);
            }
        }
        None
    }
    fn proc_tick(&self, tasks: Task<Self::Data>) {}
}

pub type SchedulerImpl = StrideScheduler;

pub fn create_scheduler() -> SchedulerImpl {
    StrideScheduler {
        last: Cell::new(None),
    }
}
