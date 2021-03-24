use super::addr::*;
use super::frame::Frame;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
pub trait VMObject: Send + Sync {
    fn page_count(&self) -> usize;
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum>;
}

pub struct VMObjectPhysical {
    base_page: PhysPageNum,
    page_count: usize,
}

impl VMObjectPhysical {
    pub fn from_range(base: PhysAddr, end: PhysAddr) -> Arc<VMObjectPhysical> {
        let base_page = base.floor_page_num();
        let end_page = end.ceil_page_num();
        Arc::new(VMObjectPhysical {
            base_page,
            page_count: usize::from(end_page) - usize::from(base_page),
        })
    }
}

impl VMObject for VMObjectPhysical {
    fn page_count(&self) -> usize {
        self.page_count
    }
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum> {
        if page_index >= self.page_count {
            return None;
        }
        Some((usize::from(self.base_page) + page_index).into())
    }
}

lazy_static! {
    pub static ref TRAMPOLINE: Arc<VMObjectPhysical> = {
        extern "C" {
            fn strampoline();
            fn etrampoline();
        }
        VMObjectPhysical::from_range((strampoline as usize).into(), (etrampoline as usize).into())
    };
}

#[allow(dead_code)]
pub struct VMObjectPaged {
    frames: Vec<Frame>,
}

impl VMObjectPaged {
    pub fn new(page_count: usize) -> Self {
        let mut frames = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            frames.push(Frame::alloc_zeroes().unwrap())
        }
        Self { frames }
    }
}
impl VMObject for VMObjectPaged {
    fn page_count(&self) -> usize {
        self.frames.len()
    }
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum> {
        self.frames.get(page_index).map(|f| f.ppn())
    }
}
