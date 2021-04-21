use super::{Task, TaskStatus};
use core::cell::Cell;
pub(super) trait Scheduler {
    type Data: Default;
    fn pick_next(&self, tasks: &[Task<Self::Data>]) -> Option<usize>;
    fn proc_tick(&self, task: &Task<Self::Data>);
}

pub(super) struct StrideScheduler;
#[derive(Default)]
pub(super) struct StrideSchedulerData {
    stride: Cell<usize>,
}

impl Scheduler for StrideScheduler {
    type Data = StrideSchedulerData;
    fn pick_next(&self, tasks: &[Task<Self::Data>]) -> Option<usize> {
        let mut current_candidate = None;
        let mut current_min: Option<usize> = None;
        for (id, task) in tasks.iter().enumerate() {
            if matches!(
                task.inner.lock().status,
                TaskStatus::UnInit | TaskStatus::Ready | TaskStatus::Running
            ) {
                let stride = task.inner.lock().sched_data.stride.get();
                let update = if let Some(min) = current_min {
                    (min.wrapping_sub(stride) as isize) > 0
                } else {
                    true
                };
                if update {
                    current_candidate = Some(id);
                    current_min = Some(stride)
                }
            }
        }
        current_candidate
    }
    fn proc_tick(&self, task: &Task<Self::Data>) {
        let priority = task.inner.lock().priority.clamp(2, isize::MAX as usize);
        task.inner
            .lock()
            .sched_data
            .stride
            .update(|f| f.wrapping_add(usize::MAX / priority));
    }
}

pub(super) type SchedulerImpl = StrideScheduler;

pub(super) fn create_scheduler() -> SchedulerImpl {
    StrideScheduler
}
