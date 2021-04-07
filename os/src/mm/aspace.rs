use lazy_static::{initialize, lazy_static};

use super::addr::*;
use super::page_table::{PTEFlags, PageTable};
use super::vmo::{VMObject, VMObjectPhysical, TRAMPOLINE};
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::*;
use riscv::register::{satp, sstatus};
use spin::Mutex;

lazy_static! {
    pub static ref TRAMPOLINE_BASE_VPN: VirtPageNum = {
        extern "C" {
            fn strampoline();
            fn etrampoline();
        }
        VirtAddr::from(0usize.wrapping_sub(etrampoline as usize - strampoline as usize))
            .floor_page_num()
    };
    pub static ref KSTACK_BASE_VPN: VirtPageNum = {
        use crate::loader::KERNEL_STACK_SIZE;
        let trampoline_va: usize = TRAMPOLINE_BASE_VPN.addr().into();
        VirtAddr::from(trampoline_va - PAGE_SIZE - KERNEL_STACK_SIZE).floor_page_num()
    };
}

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

impl AddressSpaceInner {
    fn find_mapping(&self, vaddr: VirtAddr, flags: PTEFlags) -> Option<&Arc<VMMapping>> {
        let page = vaddr.floor_page_num();
        for mapping in &self.mappings {
            if !mapping.flags.contains(flags) {
                continue;
            }
            if (page >= mapping.base_vpn)
                && (usize::from(page) < usize::from(mapping.base_vpn) + mapping.page_count)
            {
                return Some(mapping);
            }
        }
        None
    }
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
        let page_count = page_count.unwrap_or_else(|| vmo.page_count() - vmo_page_offset);
        if vmo_page_offset + page_count > vmo.page_count() {
            return None;
        }
        // check for overlapping mapping
        for mapping in &inner.mappings {
            if (usize::from(mapping.base_vpn) < usize::from(base_vpn) + page_count)
                && (usize::from(base_vpn) < usize::from(mapping.base_vpn) + mapping.page_count)
            {
                return None;
            }
        }
        inner.mappings.push(Arc::new(VMMapping {
            flags,
            base_vpn,
            page_count,
            vmo_page_offset,
            vmo: vmo.clone(),
        }));
        let mut page_table = self.page_table.lock();
        for i in 0..page_count {
            page_table.map(
                (usize::from(base_vpn) + i).into(),
                vmo.get_page(vmo_page_offset + i).unwrap(),
                flags,
            )
        }
        Some(())
    }
    pub fn read(&self, vaddr: VirtAddr, buf: &mut [u8], user: bool) -> usize {
        let inner = self.inner.lock();
        let mut flags = PTEFlags::R;
        if user {
            flags.insert(PTEFlags::U)
        }
        let mut progress = 0;
        while progress < buf.len() {
            let pos = VirtAddr::from(usize::from(vaddr) + progress);
            let mapping = match inner.find_mapping(pos, flags) {
                Some(mapping) => mapping,
                None => break,
            };
            let start = usize::from(pos) - usize::from(mapping.base_vpn.addr());
            let end = usize::min(buf.len() - progress + start, mapping.page_count * PAGE_SIZE);
            let target = progress + (end - start);
            progress += mapping.vmo.read(
                mapping.vmo_page_offset * PAGE_SIZE + start,
                &mut buf[progress..target],
            );
            if progress < target {
                break;
            }
        }
        progress
    }
    #[allow(dead_code)]
    pub fn write(&self, vaddr: VirtAddr, buf: &[u8], user: bool) -> usize {
        let inner = self.inner.lock();
        let mut flags = PTEFlags::W;
        if user {
            flags.insert(PTEFlags::U)
        }
        let mut progress = 0;
        while progress < buf.len() {
            let pos = VirtAddr::from(usize::from(vaddr) + progress);
            let mapping = match inner.find_mapping(pos, flags) {
                Some(mapping) => mapping,
                None => break,
            };
            let start = usize::from(pos) - usize::from(mapping.base_vpn.addr());
            let end = usize::min(buf.len() - progress + start, mapping.page_count * PAGE_SIZE);
            let target = progress + (end - start);
            progress += mapping.vmo.write(
                mapping.vmo_page_offset * PAGE_SIZE + start,
                &buf[progress..target],
            );
            if progress < target {
                break;
            }
        }
        progress
    }
}

lazy_static! {
    pub static ref KERNEL_ASPACE: Arc<AddressSpace> = AddressSpace::new();
}

pub fn kernel_aspace_init() {
    // for now
    unsafe { sstatus::set_sum() };
    initialize(&KERNEL_ASPACE);
    info!("setup kernel address space...");
    // kernel address space:
    // - MEMORY_START=BASE_ADDRESS=stext
    // | text, RX
    extern "C" {
        fn stext();
        fn etext();
    }
    let stext = stext as usize;
    let etext = etext as usize;
    info!("map text: {:#x?}-{:#x?}", stext, etext);
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
    info!("map rodata: {:#x?}-{:#x?}", srodata, erodata);
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
    info!("map data: {:#x?}-{:#x?}", sdata, edata);
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
    info!("map stack: {:#x?}-{:#x?}", sstack, estack);
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
    // | bss (and sbss), RW and U for now(user stack)
    // - ekernel=ebss
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let sbss = sbss as usize;
    let ebss = ebss as usize;
    info!("map bss: {:#x?}-{:#x?}", sbss, ebss);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sbss), PhysAddr::from(ebss)),
        0,
        VirtAddr::from(sbss).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W,
    );
    // | empty space
    // - DEBUGINFO_ELF_ADDRESS
    // | debug_info, R
    use crate::backtrace::{DEBUGINFO_ELF_ADDRESS, DEBUGINFO_ELF_SIZE};
    let sdebuginfo = DEBUGINFO_ELF_ADDRESS;
    let edebuginfo = DEBUGINFO_ELF_ADDRESS + DEBUGINFO_ELF_SIZE;
    info!("map debuginfo: {:#x?}-{:#x?}", sdebuginfo, edebuginfo);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sdebuginfo), PhysAddr::from(edebuginfo)),
        0,
        VirtAddr::from(sdebuginfo).floor_page_num(),
        None,
        PTEFlags::R,
    );
    // - DEBUGINFO_ELF_ADDRESS+DEBUGINFO_ELF_SIZE=FRAME_MEMORY_START
    // | frame allocated mmory, RW
    use super::frame::FRAME_MEMORY_START;
    let sframe = FRAME_MEMORY_START;
    let eframe = MEMORY_END;
    info!("map frames: {:#x?}-{:#x?}", sframe, eframe);
    KERNEL_ASPACE.map(
        VMObjectPhysical::from_range(PhysAddr::from(sframe), PhysAddr::from(eframe)),
        0,
        VirtAddr::from(sframe).floor_page_num(),
        None,
        PTEFlags::R | PTEFlags::W,
    );
    // - MEMORY_END

    // the trampoline should be special mapped to enable address space sawpping
    KERNEL_ASPACE.map(
        TRAMPOLINE.clone(),
        0,
        *TRAMPOLINE_BASE_VPN,
        None,
        PTEFlags::R | PTEFlags::X,
    );
    info!("paging enabling...");
    KERNEL_ASPACE.apply();
    info!("paging enabled!");
}
