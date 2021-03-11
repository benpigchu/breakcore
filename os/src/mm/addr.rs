use derive_more::{From, Into};

pub const MEMORY_START: usize = 0x80000000; //128MiB
pub const MEMORY_END: usize = 0x88000000; //128MiB

pub const PAGE_SIZE: usize = 0x1000;
#[allow(dead_code)]
pub const PAGE_SIZE_LOG2: usize = 12;

#[derive(Copy, Clone, Debug, From, Into, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Debug, From, Into, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Debug, From, Into, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

#[derive(Copy, Clone, Debug, From, Into, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

pub trait Addr {
    type PageNum;
    fn floor_page_num(&self) -> Self::PageNum;
    fn ceil_page_num(&self) -> Self::PageNum;
    fn page_offset(&self) -> usize;
    fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

pub trait AddrBase {
    type PageNum;
}

impl AddrBase for PhysAddr {
    type PageNum = PhysPageNum;
}

impl AddrBase for VirtAddr {
    type PageNum = VirtPageNum;
}

impl<A: AddrBase> Addr for A
where
    usize: From<A>,
    A: Copy,
    <A as AddrBase>::PageNum: From<usize>,
{
    type PageNum = <A as AddrBase>::PageNum;
    fn floor_page_num(&self) -> Self::PageNum {
        (usize::from(*self) / PAGE_SIZE).into()
    }
    fn ceil_page_num(&self) -> Self::PageNum {
        (usize::from(*self) / PAGE_SIZE + if self.aligned() { 0 } else { 1 }).into()
    }
    fn page_offset(&self) -> usize {
        usize::from(*self) % PAGE_SIZE
    }
}

pub trait PageNumBase {
    type Addr;
}

impl PageNumBase for PhysPageNum {
    type Addr = PhysAddr;
}

impl PageNumBase for VirtPageNum {
    type Addr = VirtAddr;
}

pub trait PageNum {
    type Addr;
    fn addr(&self) -> Self::Addr;
}

impl<PN: PageNumBase> PageNum for PN
where
    usize: From<PN>,
    PN: Copy,
    <PN as PageNumBase>::Addr: From<usize>,
{
    type Addr = <PN as PageNumBase>::Addr;
    fn addr(&self) -> Self::Addr {
        (usize::from(*self) * PAGE_SIZE).into()
    }
}

impl PhysAddr {
    pub fn get_mut<T>(&self) -> *mut T {
        usize::from(*self) as *mut T
    }
}

impl VirtPageNum {
    /// Used by page table
    pub fn indexes(&self) -> [usize; 3] {
        [
            (usize::from(*self) >> 18) % (1usize << 9),
            (usize::from(*self) >> 9) % (1usize << 9),
            usize::from(*self) % (1usize << 9),
        ]
    }
}
