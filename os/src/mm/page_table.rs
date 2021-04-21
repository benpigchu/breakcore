#![allow(dead_code)]
use core::ops::{Deref, DerefMut};

use bitflags::bitflags;

use super::addr::*;
use super::frame::Frame;
bitflags! {
    /// Flags in the PTE
    /// Note on RWX:
    /// - W but not R is reserved (And should not be used for now)
    /// - not R and W and X means this point to next level of page table,
    ///   unset the V for the real not RWX
    pub struct PTEFlags: u8 {
        /// Valid
        const V = 1 << 0;
        /// Read
        const R = 1 << 1;
        /// Write
        const W = 1 << 2;
        /// eXecute
        const X = 1 << 3;
        /// User
        const U = 1 << 4;
        /// Global, not used in our implementation
        const G = 1 << 5;
        /// Access
        const A = 1 << 6;
        /// Dirty
        const D = 1 << 7;
        /// Helper for extraction RWX bits
        const RWX =Self::R.bits|Self::W.bits|Self::X.bits;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    /// The bits from high to low:
    /// - 10 bits reserved ([63:54])
    /// - 26 bits PPN[2] ([53:28])
    /// - 9 bits PPN[1] ([27:19])
    /// - 9 bits PPN[0] ([18:10])
    ///   Note: to be simple, in our implementation there are no superpage
    ///   So PPN[2] to PPN[0] is a single PPN
    /// - 2 bits RSW ([9:8])
    ///   which is ignored by the hardware
    /// - 8 bits PTEFlags ([7:0])
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: usize::from(ppn) << 10 | flags.bits() as usize,
        }
    }
    pub fn new_empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        ((self.bits >> 10) % (1usize << 44)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.bits as u8)
    }
    pub fn valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }
    pub fn readable(&self) -> bool {
        self.flags().contains(PTEFlags::R)
    }
    pub fn writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }
    pub fn executable(&self) -> bool {
        self.flags().contains(PTEFlags::X)
    }
}

pub struct RawPageTable {
    root: PhysPageNum,
}

impl RawPageTable {
    fn find_pte(&self, vpn: VirtPageNum, create_when_absent: bool) -> Option<&mut PageTableEntry> {
        let mut ppn = self.root;
        for (level, &vpni) in vpn.indexes().iter().enumerate() {
            let ptes = unsafe {
                ppn.addr()
                    .get_mut::<[PageTableEntry; 0x200]>()
                    .as_mut()
                    .unwrap()
            };
            let pte = &mut ptes[vpni];
            if level == 2 {
                return Some(pte);
            }
            if !pte.valid() {
                if create_when_absent {
                    let frame = Frame::alloc_zeroes().unwrap();
                    *pte = PageTableEntry::new(frame.ppn(), PTEFlags::V);
                    // We track the frame in the page table itself,
                    // so we need to bypass the drop of the frame
                    core::mem::forget(frame)
                } else {
                    break;
                }
            }
            ppn = pte.ppn();
        }
        None
    }
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte(vpn, true).unwrap();
        assert!(!pte.valid(), "Mapping a mapped page, vpn:{:#x?}", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V)
    }
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn, true).unwrap();
        assert!(pte.valid(), "Unmapping a not mapped page, vpn:{:#x?}", vpn);
        *pte = PageTableEntry::new_empty()
    }
    pub fn query(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn, false).map(|pte| *pte)
    }
    pub fn token(&self) -> usize {
        8usize << 60 | usize::from(self.root)
    }
}

pub struct PageTable {
    raw: RawPageTable,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = Frame::alloc_zeroes().unwrap();
        let ppn = frame.ppn();
        // We track the frame in the page table itself,
        // so we need to bypass the drop of the frame
        core::mem::forget(frame);
        PageTable {
            raw: RawPageTable { root: ppn },
        }
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        fn dealloc_ptes_page(ppn: PhysPageNum) {
            let ptes = unsafe {
                ppn.addr()
                    .get_mut::<[PageTableEntry; 0x200]>()
                    .as_mut()
                    .unwrap()
            };
            for pte in ptes {
                if pte.valid() && (pte.flags() & PTEFlags::RWX).is_empty() {
                    dealloc_ptes_page(pte.ppn())
                }
            }
            Frame::manually_drop(ppn);
        }
        dealloc_ptes_page(self.raw.root);
    }
}

impl Deref for PageTable {
    type Target = RawPageTable;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

pub struct PageTableRef {
    raw: RawPageTable,
}

impl PageTableRef {
    pub fn from_token(satp: usize) -> Self {
        let ppn = PhysPageNum::from(satp % (1usize << 44));
        PageTableRef {
            raw: RawPageTable { root: ppn },
        }
    }
}

impl Deref for PageTableRef {
    type Target = RawPageTable;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
