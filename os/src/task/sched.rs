use super::{Task, TaskStatus};
use alloc::sync::Arc;
pub(super) trait Scheduler {
    type Data: Default;
    fn pick_next(&self, tasks: &[Arc<Task<Self::Data>>]) -> Option<usize>;
    fn proc_tick(&self, task: &Arc<Task<Self::Data>>);
}

pub(super) struct StrideScheduler;
#[derive(Default)]
pub(super) struct StrideSchedulerData {
    stride: usize,
}

impl Scheduler for StrideScheduler {
    type Data = StrideSchedulerData;
    fn pick_next(&self, tasks: &[Arc<Task<Self::Data>>]) -> Option<usize> {
        let mut current_candidate = None;
        let mut current_min: Option<usize> = None;
        for (id, task) in tasks.iter().enumerate() {
            let task_inner = task.inner.lock();
            if matches!(task_inner.status, TaskStatus::Ready | TaskStatus::Running) {
                let stride = task_inner.sched_data.stride;
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
    fn proc_tick(&self, task: &Arc<Task<Self::Data>>) {
        let mut task_inner = task.inner.lock();
        let priority = task_inner.priority.clamp(2, isize::MAX as usize);
        let stride = &mut task_inner.sched_data.stride;
        *stride = stride.wrapping_add(usize::MAX / priority);
    }
}

pub(super) type SchedulerImpl = StrideScheduler;

pub(super) fn create_scheduler() -> SchedulerImpl {
    StrideScheduler
}
