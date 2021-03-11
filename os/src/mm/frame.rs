use super::addr::*;
use lazy_static::{initialize, lazy_static};
use spin::Mutex;

pub const FRAME_MEMORY_START: usize = 0x82000000; //128MiB

#[derive(Debug)]
pub struct Frame {
    ppn: PhysPageNum,
}

impl Frame {
    pub fn alloc_zeroes() -> Option<Self> {
        Self::alloc_uninitialized().map(|frame| {
            unsafe {
                frame
                    .ppn
                    .addr()
                    .get_mut::<[u8; PAGE_SIZE]>()
                    .as_mut()
                    .unwrap()
                    .fill(0)
            };
            frame
        })
    }
    pub fn alloc_uninitialized() -> Option<Self> {
        Some(Frame {
            ppn: FRAME_ALLOCATOR.lock().alloc()?,
        })
    }
    pub fn ppn(&self) -> PhysPageNum {
        self.ppn
    }
    pub fn manually_drop(ppn: PhysPageNum) {
        FRAME_ALLOCATOR.lock().dealloc(ppn)
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self.ppn);
    }
}

trait FrameAllocator {
    fn new(start: PhysPageNum, end: PhysPageNum) -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
    fn check_allocated(&self, ppn: PhysPageNum) -> bool;
}

pub struct LinkedStackFrameAllocator {
    current: PhysPageNum,
    end: PhysPageNum,
    recycled_head: Option<PhysPageNum>,
}

impl FrameAllocator for LinkedStackFrameAllocator {
    fn new(start: PhysPageNum, end: PhysPageNum) -> Self {
        LinkedStackFrameAllocator {
            current: start,
            end,
            recycled_head: None,
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled_head {
            let next_ptr: *mut Option<PhysPageNum> = ppn.addr().get_mut::<Option<PhysPageNum>>();
            self.recycled_head = unsafe { next_ptr.read_volatile() };
            return Some(ppn);
        } else if self.current < self.end {
            let current = self.current;
            self.current = (usize::from(current) + 1).into();
            return Some(current);
        }
        None
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        if !self.check_allocated(ppn) {
            panic!("Dealloc a not allocated page: ppn={:#x?}", ppn)
        }
        let next_ptr: *mut Option<PhysPageNum> = ppn.addr().get_mut::<Option<PhysPageNum>>();
        unsafe { next_ptr.write_volatile(self.recycled_head) };
        self.recycled_head = Some(ppn)
    }
    fn check_allocated(&self, ppn: PhysPageNum) -> bool {
        if usize::from(ppn) >= usize::from(self.current) {
            return false;
        }
        let mut next = self.recycled_head;
        while let Some(pagenum) = next {
            if pagenum == ppn {
                return false;
            }
            let next_ptr: *mut Option<PhysPageNum> =
                pagenum.addr().get_mut::<Option<PhysPageNum>>();
            next = unsafe { next_ptr.read_volatile() };
        }
        true
    }
}

type FrameAllocatorImpl = LinkedStackFrameAllocator;

lazy_static! {
    static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> = Mutex::new(FrameAllocatorImpl::new(
        PhysAddr::from(FRAME_MEMORY_START).floor_page_num(),
        PhysAddr::from(MEMORY_END).ceil_page_num()
    ));
}

pub fn init() {
    initialize(&FRAME_ALLOCATOR);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    // Note: this tests internal behaviour of the LinkedStackFrameAllocator
    let pn1 = FRAME_ALLOCATOR.lock().alloc().unwrap();
    let pn2 = FRAME_ALLOCATOR.lock().alloc().unwrap();
    let pn3 = FRAME_ALLOCATOR.lock().alloc().unwrap();
    let pn3p1 = PhysPageNum::from(usize::from(pn3) + 1);
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    println!("[kernel] frame_allocator_test: alloc:pn1={:#x?} pn2={:#x?} pn3={:#x?} not alloc: pn3+1={:#x?}",pn1,pn2,pn3,pn3p1);
    // now pn2=pn1+1,pn3=pn2+1
    FRAME_ALLOCATOR.lock().dealloc(pn1);
    FRAME_ALLOCATOR.lock().dealloc(pn3);
    // now pn1 pn3 is in the recycle list
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    println!("[kernel] frame_allocator_test: dealloc pn1 pn3");
    let mut pn3re = FRAME_ALLOCATOR.lock().alloc().unwrap();
    let mut pn1re = FRAME_ALLOCATOR.lock().alloc().unwrap();
    assert_eq!(pn3re, pn3);
    assert_eq!(pn1re, pn1);
    println!("[kernel] frame_allocator_test: alloc pn3 pn1 again");
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    FRAME_ALLOCATOR.lock().dealloc(pn2);
    FRAME_ALLOCATOR.lock().dealloc(pn3);
    FRAME_ALLOCATOR.lock().dealloc(pn1);
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    println!("[kernel] frame_allocator_test: dealloc pn2 pn3 pn1");
    let mut pn1re2 = FRAME_ALLOCATOR.lock().alloc().unwrap();
    assert_eq!(pn1re2, pn1);
    assert!(FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    println!("[kernel] frame_allocator_test: alloc pn1 again");
    FRAME_ALLOCATOR.lock().dealloc(pn1);
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn1));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn2));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3));
    assert!(!FRAME_ALLOCATOR.lock().check_allocated(pn3p1));
    println!("[kernel] frame_allocator_test: dealloc pn1");
    panic!("frame_allocator_test_passed!");
}
