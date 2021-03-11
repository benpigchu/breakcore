use lazy_static::{initialize, lazy_static};

use super::addr::*;
use super::page_table::{PTEFlags, PageTable};
use alloc::sync::Arc;
use riscv::register::{satp, sstatus};
use spin::Mutex;

pub struct AddressSpace {
    page_table: PageTable,
}

impl AddressSpace {
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
        }
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    pub fn apply(&self) {
        let satp = self.token();
        unsafe {
            satp::write(satp);
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }
    pub fn map_range(
        &mut self,
        base_vpn: VirtPageNum,
        base_ppn: PhysPageNum,
        count: usize,
        flags: PTEFlags,
    ) {
        for i in 0..count {
            self.page_table.map(
                (usize::from(base_vpn) + i).into(),
                (usize::from(base_ppn) + i).into(),
                flags,
            )
        }
    }
}

lazy_static! {
    pub static ref KERNEL_ASPACE: Arc<Mutex<AddressSpace>> =
        Arc::new(Mutex::new(AddressSpace::new()));
}

pub fn kernel_aspace_init() {
    // for now
    unsafe { sstatus::set_sum() };
    initialize(&KERNEL_ASPACE);
    println!("[kernel] setup kernel address space...");
    let mut kernel_aspace = KERNEL_ASPACE.lock();
    let base_ppn = PhysAddr::from(MEMORY_START).floor_page_num();
    let end_ppn = PhysAddr::from(MEMORY_END).ceil_page_num();
    kernel_aspace.map_range(
        VirtPageNum::from(usize::from(base_ppn)),
        base_ppn,
        usize::from(end_ppn) - usize::from(base_ppn),
        PTEFlags::RWX,
    );
    test_kernel_aspece(&kernel_aspace);
    println!("[kernel] paging enabling...");
    kernel_aspace.apply();
    println!("[kernel] paging enabled!");
}

fn test_kernel_aspece(kernel_aspace: &AddressSpace) {
    // A simple test of a page in the memory is mapped
    let vpn = VirtAddr::from((MEMORY_START + MEMORY_END) / 2).floor_page_num();
    let entry = kernel_aspace.page_table.query(vpn).unwrap();
    assert!(entry.valid());
    assert_eq!(usize::from(entry.ppn()), usize::from(vpn));
}
