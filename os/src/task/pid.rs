use super::TaskContext;
use crate::mm::addr::*;
use crate::mm::aspace::{KERNEL_ASPACE, TRAMPOLINE_BASE_VPN};
use crate::mm::vmo::VMObjectPaged;
use crate::mm::PTEFlags;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct PidHandle(usize);

impl PidHandle {
    pub fn alloc() -> Self {
        let pid = PID_ALLOCATOR.lock().alloc();
        log::info!("alloc pid: {:#x?}", pid);
        // map kernel stack
        let kstack_vmo = VMObjectPaged::new(page_count(KERNEL_STACK_SIZE)).unwrap();
        let vskstack = kernel_stack_addr(pid);
        KERNEL_ASPACE.map(
            kstack_vmo,
            0,
            VirtAddr::from(vskstack).floor_page_num(),
            None,
            PTEFlags::R | PTEFlags::W,
        );
        let kstack = unsafe { (vskstack as *mut KernelStack).as_mut().unwrap() };
        kstack.push_context(TaskContext::goto_launch());
        PidHandle(pid)
    }
    pub fn value(&self) -> usize {
        self.0
    }
    fn kernel_stack_addr(&self) -> usize {
        kernel_stack_addr(self.0)
    }
    pub fn kernel_stack(&self) -> &'static KernelStack {
        unsafe {
            (self.kernel_stack_addr() as *mut KernelStack)
                .as_mut()
                .unwrap()
        }
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        // unmap kernel stack
        log::info!("drop pid: {:#x?}", self.0);
        KERNEL_ASPACE.unmap(
            VirtAddr::from(self.kernel_stack_addr()).floor_page_num(),
            page_count(core::mem::size_of::<KernelStack>()),
            false,
        );
        PID_ALLOCATOR.lock().dealloc(self.0);
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
        !self.recycled.contains(&pid)
    }
}

type PidAllocatorImpl = StackPidAllocator;

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocatorImpl> = Mutex::new(Default::default());
}

pub const KERNEL_STACK_SIZE: usize = 4096 * 16;
#[repr(align(4096))]
pub struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    pub fn get_bottom_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn get_init_sp(&self) -> usize {
        self.get_bottom_sp() - core::mem::size_of::<TaskContext>()
    }
    pub fn push_context(&self, task_cx: TaskContext) {
        let task_cx_ptr = (self.get_bottom_sp() as usize - core::mem::size_of::<TaskContext>())
            as *mut TaskContext;
        unsafe {
            *task_cx_ptr = task_cx;
        }
    }
}

fn kernel_stack_addr(pid: usize) -> usize {
    usize::from(TRAMPOLINE_BASE_VPN.addr()) - (KERNEL_STACK_SIZE + PAGE_SIZE) * (pid + 1)
}
