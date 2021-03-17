use super::addr::*;
pub trait VMObject {
    fn page_count(&self) -> usize;
    fn get_page(&self, page_index: usize) -> Option<PhysPageNum>;
}

pub struct VMObjectPhysical {
    base_page: PhysPageNum,
    page_count: usize,
}

impl VMObjectPhysical {
    pub fn from_range(base: PhysAddr, end: PhysAddr) -> VMObjectPhysical {
        let base_page = base.floor_page_num();
        let end_page = end.ceil_page_num();
        VMObjectPhysical {
            base_page,
            page_count: usize::from(end_page) - usize::from(base_page),
        }
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
