use crate::mm::addr::*;
use crate::mm::vmo::VMObjectPaged;
use crate::mm::PTEFlags;
use crate::task::TASK_MANAGER;
use bitflags::bitflags;

bitflags! {
    pub struct MMapprot: usize {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

pub fn sys_mmap(start: VirtAddr, len: usize, prot: usize) -> isize {
    let aspace = if let Some(aspace) = TASK_MANAGER.current_aspace() {
        aspace
    } else {
        return -1;
    };
    let prot = if let Some(prot) = MMapprot::from_bits(prot) {
        prot
    } else {
        return -1;
    };
    if prot.is_empty() {
        return -1;
    }
    if !start.aligned() {
        return -1;
    }
    let page_count = page_count(len);
    let mut flags = PTEFlags::U;
    if prot.contains(MMapprot::READ) {
        flags.insert(PTEFlags::R)
    }
    if prot.contains(MMapprot::WRITE) {
        flags.insert(PTEFlags::W)
    }
    if prot.contains(MMapprot::EXEC) {
        flags.insert(PTEFlags::X)
    }
    let vmo = if let Some(vmo) = VMObjectPaged::new(page_count) {
        vmo
    } else {
        return -1;
    };
    match aspace.map(vmo, 0, start.floor_page_num(), None, flags) {
        Some(_) => (page_count * PAGE_SIZE) as isize,
        None => -1,
    }
}

pub fn sys_munmap(start: VirtAddr, len: usize) -> isize {
    let aspace = if let Some(aspace) = TASK_MANAGER.current_aspace() {
        aspace
    } else {
        return -1;
    };
    if !start.aligned() {
        return -1;
    }
    let page_count = page_count(len);
    match aspace.unmap(start.floor_page_num(), page_count, true) {
        Some(_) => (page_count * PAGE_SIZE) as isize,
        None => -1,
    }
}
