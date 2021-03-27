use super::addr::*;
use super::frame::Frame;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::slice;
use lazy_static::lazy_static;
pub trait VMObject: Send + Sync {
    fn page_count(&self) -> usize;
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum>;
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize;
    fn write(&self, offset: usize, buf: &[u8]) -> usize;
}

pub struct VMObjectPhysical {
    base_page: PhysPageNum,
    page_count: usize,
}

impl VMObjectPhysical {
    pub fn from_range(base: PhysAddr, end: PhysAddr) -> Arc<Self> {
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
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let limit = self.page_count() * PAGE_SIZE;
        if offset > limit {
            return 0;
        }
        let len = usize::min(buf.len(), limit - offset);
        let slice = unsafe {
            slice::from_raw_parts(
                (usize::from(self.base_page.addr()) + offset) as *const u8,
                len,
            )
        };
        buf[..len].copy_from_slice(slice);
        len
    }
    fn write(&self, offset: usize, buf: &[u8]) -> usize {
        let limit = self.page_count() * PAGE_SIZE;
        if offset > limit {
            return 0;
        }
        let len = usize::min(buf.len(), limit - offset);
        let slice = unsafe {
            slice::from_raw_parts_mut(
                (usize::from(self.base_page.addr()) + offset) as *mut u8,
                len,
            )
        };
        slice.copy_from_slice(&buf[..len]);
        len
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
    pub fn new(page_count: usize) -> Arc<Self> {
        let mut frames = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            frames.push(Frame::alloc_zeroes().unwrap())
        }
        Arc::new(Self { frames })
    }
}
impl VMObject for VMObjectPaged {
    fn page_count(&self) -> usize {
        self.frames.len()
    }
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum> {
        self.frames.get(page_index).map(|f| f.ppn())
    }
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let limit = self.page_count() * PAGE_SIZE;
        if offset > limit {
            return 0;
        }
        let len = usize::min(buf.len(), limit - offset);
        let start_page = offset / PAGE_SIZE;
        let mut current_page = start_page;
        let mut progress = 0usize;
        while progress < len {
            let start = current_page * PAGE_SIZE - offset - progress;
            let end = usize::min(len - progress, PAGE_SIZE);
            let target = progress + (end - start);
            buf[progress..target].copy_from_slice(&self.frames[current_page].content()[start..end]);
            progress = target;
            current_page += 1;
        }
        len
    }
    fn write(&self, offset: usize, buf: &[u8]) -> usize {
        let limit = self.page_count() * PAGE_SIZE;
        if offset > limit {
            return 0;
        }
        let len = usize::min(buf.len(), limit - offset);
        let start_page = offset / PAGE_SIZE;
        let mut current_page = start_page;
        let mut progress = 0usize;
        while progress < len {
            let start = current_page * PAGE_SIZE - offset - progress;
            let end = usize::min(len - progress, PAGE_SIZE);
            let target = progress + (end - start);
            self.frames[current_page].content()[start..end].copy_from_slice(&buf[progress..target]);
            progress = target;
            current_page += 1;
        }
        len
    }
}
