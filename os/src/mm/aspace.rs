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
    // kernel address space:
    // - MEMORY_START=BASE_ADDRESS=stext
    // | text, RX
    extern "C" {
        fn stext();
        fn etext();
    }
    let stext = stext as usize;
    let etext = etext as usize;
    println!("[kernel] map text: {:#x?}-{:#x?}", stext, etext);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(stext), PhysAddr::from(etext)),
        0,
        VirtAddr::from(stext).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::X,
    );
    // - etext=srodata
    // | rodata, R
    extern "C" {
        fn srodata();
        fn erodata();
    }
    let srodata = srodata as usize;
    let erodata = erodata as usize;
    println!("[kernel] map rodata: {:#x?}-{:#x?}", srodata, erodata);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(srodata), PhysAddr::from(erodata)),
        0,
        VirtAddr::from(srodata).floor_page_num(),
        None,
        PTEFlags::R,
    );
    // - erodata=sdata
    // | data, RW
    extern "C" {
        fn sdata();
        fn edata();
    }
    let sdata = sdata as usize;
    let edata = edata as usize;
    println!("[kernel] map data: {:#x?}-{:#x?}", sdata, edata);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sdata), PhysAddr::from(edata)),
        0,
        VirtAddr::from(sdata).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W,
    );
    // - edata
    // | empty space for stack overflow protect
    // - sstack
    // | launch stack, RW
    extern "C" {
        fn sstack();
        fn estack();
    }
    let sstack = sstack as usize;
    let estack = estack as usize;
    println!("[kernel] map stack: {:#x?}-{:#x?}", sstack, estack);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sstack), PhysAddr::from(estack)),
        0,
        VirtAddr::from(sstack).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W,
    );
    // - estack
    // | empty space for stack overflow protect
    // - sbss
    // | bss, RW and U for now(user stack)
    // - ekernel=ebss
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let sbss = sbss as usize;
    let ebss = ebss as usize;
    println!("[kernel] map bss: {:#x?}-{:#x?}", sbss, ebss);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sbss), PhysAddr::from(ebss)),
        0,
        VirtAddr::from(sbss).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W | PTEFlags::U,
    );
    // | empty space
    // - DEBUGINFO_ELF_ADDRESS
    // | debug_info, R
    use crate::backtrace::{DEBUGINFO_ELF_ADDRESS, DEBUGINFO_ELF_SIZE};
    let sdebuginfo = DEBUGINFO_ELF_ADDRESS;
    let edebuginfo = DEBUGINFO_ELF_ADDRESS + DEBUGINFO_ELF_SIZE;
    println!(
        "[kernel] map debuginfo: {:#x?}-{:#x?}",
        sdebuginfo, edebuginfo
    );
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sdebuginfo), PhysAddr::from(edebuginfo)),
        0,
        VirtAddr::from(sdebuginfo).floor_page_num(),
        None,
        PTEFlags::R,
    );
    // - DEBUGINFO_ELF_ADDRESS+DEBUGINFO_ELF_SIZE=APP_BASE_ADDRESS
    // | app memory space, RWXU
    use crate::loader::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, MAX_APP_NUM};
    let sapp = *APP_BASE_ADDRESS;
    let eapp = sapp + MAX_APP_NUM * (*APP_SIZE_LIMIT);
    println!("[kernel] map app: {:#x?}-{:#x?}", sapp, eapp);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sapp), PhysAddr::from(eapp)),
        0,
        VirtAddr::from(sapp).floor_page_num(),
        None,
        PTEFlags::RWX | PTEFlags::U,
    );
    // - APP_BASE_ADDRESS+APP_SIZE_LIMIT*MAX_APP_NUM
    // | empty space
    // - FRAME_MEMORY_START
    // | frame allocated mmory, RW
    use super::frame::FRAME_MEMORY_START;
    let sframe = FRAME_MEMORY_START;
    let eframe = MEMORY_END;
    println!("[kernel] map frames: {:#x?}-{:#x?}", sframe, eframe);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sframe), PhysAddr::from(eframe)),
        0,
        VirtAddr::from(sframe).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W,
    );
    // - MEMORY_END
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
