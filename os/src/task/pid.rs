use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct PidHandle(usize);

impl PidHandle {
    pub fn alloc() -> Self {
        let pid = PID_ALLOCATOR.lock().alloc();
        // TODO: map kstack here
        PidHandle(pid)
    }
    pub fn value(&self) -> usize {
        self.0
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0)
        // TODO: unmap kstack here
    }
}

trait PidAllocator: Default {
    fn alloc(&mut self) -> usize;
    fn dealloc(&mut self, pid: usize);
    fn check_allocated(&self, pid: usize) -> bool;
}

#[derive(Default)]
pub struct StackPidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator for StackPidAllocator {
    fn alloc(&mut self) -> usize {
        if let Some(pid) = self.recycled.pop() {
            pid
        } else {
            let current = self.current;
            self.current += 1;
            current
        }
    }
    fn dealloc(&mut self, pid: usize) {
        if !self.check_allocated(pid) {
            panic!("Dealloc a not allocated pid: pid={:#x?}", pid)
        }
        self.recycled.push(pid);
    }
    fn check_allocated(&self, pid: usize) -> bool {
        if pid >= self.current {
            return false;
        }
        self.recycled.contains(&pid)
    }
}

type PidAllocatorImpl = StackPidAllocator;

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocatorImpl> = Mutex::new(Default::default());
}
