use lazy_static::{initialize, lazy_static};

use super::addr::*;
use super::page_table::{PTEFlags, PageTable};
use super::vmo::{VMObject, VMObjectPhysical};
use alloc::sync::Arc;
use alloc::vec::Vec;
use riscv::register::{satp, sstatus};
use spin::Mutex;

#[allow(dead_code)]
struct VMMapping {
    flags: PTEFlags,
    base_vpn: VirtPageNum,
    page_count: usize,
    vmo_page_offset: usize,
    vmo: Arc<dyn VMObject>,
}

pub struct AddressSpace {
    //note: always lock inner first
    page_table: Arc<Mutex<PageTable>>,
    inner: Mutex<AddressSpaceInner>,
}

struct AddressSpaceInner {
    mappings: Vec<Arc<VMMapping>>,
}

impl AddressSpace {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            page_table: Arc::new(Mutex::new(PageTable::new())),
            inner: Mutex::new(AddressSpaceInner {
                mappings: Vec::new(),
            }),
        })
    }
    pub fn token(&self) -> usize {
        self.page_table.lock().token()
    }
    pub fn apply(&self) {
        let satp = self.token();
        unsafe {
            satp::write(satp);
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }
    pub fn map(
        &self,
        vmo: Arc<dyn VMObject>,
        vmo_page_offset: usize,
        base_vpn: VirtPageNum,
        page_count: Option<usize>,
        flags: PTEFlags,
    ) -> Option<()> {
        let mut inner = self.inner.lock();
        let mut page_table = self.page_table.lock();
        let page_count = page_count.unwrap_or_else(|| vmo.page_count() - vmo_page_offset);
        if vmo_page_offset + page_count > vmo.page_count() {
            return None;
        }
        inner.mappings.push(Arc::new(VMMapping {
            flags,
            base_vpn,
            page_count,
            vmo_page_offset,
            vmo: vmo.clone(),
        }));
        for i in 0..page_count {
            page_table.map(
                (usize::from(base_vpn) + i).into(),
                vmo.get_page(vmo_page_offset + i).unwrap(),
                flags,
            )
        }
        Some(())
    }
}

lazy_static! {
    pub static ref KERNEL_ASPACE: Arc<AddressSpace> = AddressSpace::new();
}

pub fn kernel_aspace_init() {
    // for now
    unsafe { sstatus::set_sum() };
    initialize(&KERNEL_ASPACE);
    println!("[kernel] setup kernel address space...");
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(MEMORY_START), PhysAddr::from(MEMORY_END)),
        0,
        VirtAddr::from(MEMORY_START).floor_page_num(),
        None,
        PTEFlags::RWX,
    );
    // KERNEL_ASPACE.map_range(
    //     VirtPageNum::from(usize::from(base_ppn)),
    //     base_ppn,
    //     usize::from(end_ppn) - usize::from(base_ppn),
    //     PTEFlags::RWX,
    // );
    // test_kernel_aspece();
    println!("[kernel] paging enabling...");
    KERNEL_ASPACE.apply();
    println!("[kernel] paging enabled!");
}

// fn test_kernel_aspece() {
//     // A simple test of a page in the memory is mapped
//     let vpn = VirtAddr::from((MEMORY_START + MEMORY_END) / 2).floor_page_num();
//     let entry = KERNEL_ASPACE.page_table.lock().query(vpn).unwrap();
//     assert!(entry.valid());
//     assert_eq!(usize::from(entry.ppn()), usize::from(vpn));
// }
