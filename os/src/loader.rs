use crate::mm::addr::*;
use crate::mm::aspace::AddressSpace;
use crate::mm::vmo::{VMObject, VMObjectPaged};
use crate::mm::PTEFlags;
use alloc::sync::Arc;
use core::slice;
use lazy_static::*;
use log::*;

global_asm!(include_str!("embed_app.asm"));

pub const USER_STACK_SIZE: usize = 4096 * 16;
pub const MAX_APP_NUM: usize = 16;

#[derive(Default)]
struct AppInfo {
    start: usize,
    end: usize,
    name: &'static str,
}

pub struct AppManager {
    pub app_num: usize,
    apps: [AppInfo; MAX_APP_NUM],
    pub init_app: Option<&'static str>,
}

unsafe impl Sync for AppManager {}

lazy_static! {
    pub static ref APP_MANAGER: AppManager = {
        extern "C" {
            fn app_list();
        }
        let app_list = app_list as usize as *const usize;
        let app_num = unsafe { app_list.read_volatile() };
        if app_num > MAX_APP_NUM {
            panic!("Too many apps!");
        }
        let mut apps: [AppInfo; MAX_APP_NUM] = Default::default();
        for (i, app) in apps.iter_mut().enumerate().take(app_num) {
            *app = unsafe {
                let app_name_ptr = app_list.add(1 + 3 * i).read_volatile() as *const u8;
                let mut app_name_len = 0;
                while app_name_ptr.add(app_name_len).read_volatile() != 0 {
                    app_name_len += 1;
                }
                let app_name_data = slice::from_raw_parts(app_name_ptr, app_name_len);
                AppInfo {
                    start: app_list.add(2 + 3 * i).read_volatile(),
                    end: app_list.add(3 + 3 * i).read_volatile(),
                    name: core::str::from_utf8(app_name_data).unwrap(),
                }
            }
        }
        let init_app = apps.get(0).map(|app| app.name);
        AppManager {
            app_num,
            apps,
            init_app,
        }
    };
}

impl AppManager {
    pub fn print_info(&self) {
        info!("app_num: {}", APP_MANAGER.app_num);
        for i in 0..APP_MANAGER.app_num {
            info!("    {}: {}", i, APP_MANAGER.apps[i].name);
            info!(
                "        {:#x?}-{:#x?}",
                APP_MANAGER.apps[i].start, APP_MANAGER.apps[i].end
            );
        }
    }
    pub fn load_elf(&self, name: &str, aspace: &Arc<AddressSpace>) -> Option<LoadedElf> {
        let app = self.apps.iter().find(|app| app.name == name)?;
        let app_start_address = app.start;
        let app_end_address = app.end;
        let app_bin_data = unsafe {
            slice::from_raw_parts(
                app_start_address as *const u8,
                app_end_address - app_start_address,
            )
        };
        let mut elf_vaddr_end = 0usize;
        let mut stack_pte_flags = PTEFlags::U;

        use object::elf::*;
        use object::read::elf::*;
        use object::{Bytes, LittleEndian};
        let app_bin_bytes = Bytes(app_bin_data);
        // we are using little endian elf64 format
        let file_header = FileHeader64::<LittleEndian>::parse(app_bin_bytes).ok()?;
        info!("Parsing ELF for {}...", app.name);
        if !file_header.is_little_endian() {
            return None;
        }
        if !file_header.is_class_64() {
            return None;
        }
        if file_header.e_machine(LittleEndian) != EM_RISCV {
            return None;
        }
        if file_header.e_type(LittleEndian) != ET_EXEC {
            return None;
        }
        let entry = file_header.e_entry(LittleEndian) as usize;
        let program_headers = file_header
            .program_headers(LittleEndian, app_bin_bytes)
            .ok()?;
        aspace.clean_user();
        for program_header in program_headers {
            fn pte_flags_from_ph_flags(ph_flags: u32) -> PTEFlags {
                let mut pte_flags = PTEFlags::empty();
                if ph_flags & PF_R != 0 {
                    pte_flags.insert(PTEFlags::R)
                }
                if ph_flags & PF_W != 0 {
                    pte_flags.insert(PTEFlags::W)
                }
                if ph_flags & PF_X != 0 {
                    pte_flags.insert(PTEFlags::X)
                }
                pte_flags
            }
            match program_header.p_type(LittleEndian) {
                PT_LOAD => {
                    info!("    ELF segment:LOAD");
                    let ph_flags = program_header.p_flags(LittleEndian);
                    let mut pte_flags = pte_flags_from_ph_flags(ph_flags);
                    info!("        flags:{:?}", pte_flags);
                    pte_flags.insert(PTEFlags::U);
                    let vaddr_start = program_header.p_vaddr(LittleEndian) as usize;
                    let mem_size = program_header.p_memsz(LittleEndian) as usize;
                    let vaddr_end = vaddr_start + mem_size;
                    info!("        vaddr:{:#x?}-{:#x?}", vaddr_start, vaddr_end);
                    if vaddr_start % PAGE_SIZE != 0 {
                        panic!("ELF LOAD segment start address not page aligned")
                    }
                    let vmo = VMObjectPaged::new(page_count(mem_size))?;
                    let buf = program_header
                        .data_as_array(LittleEndian, app_bin_bytes)
                        .ok()?;
                    let wrote_size = vmo.write(0, buf);
                    assert_eq!(buf.len(), wrote_size);
                    aspace.map(
                        vmo,
                        0,
                        VirtAddr::from(vaddr_start).floor_page_num(),
                        None,
                        pte_flags,
                    )?;
                    elf_vaddr_end = usize::max(elf_vaddr_end, vaddr_end)
                }
                PT_GNU_STACK => {
                    info!("    ELF segment:STACK");
                    let ph_flags = program_header.p_flags(LittleEndian);
                    let pte_flags = pte_flags_from_ph_flags(ph_flags);
                    info!("        flags:{:?}", pte_flags);
                    stack_pte_flags.insert(pte_flags)
                }
                PT_INTERP => {
                    warn!("Dynamic linking is not supported");
                    return None;
                }
                other => {
                    info!("    ELF segment:{:?}", other);
                }
            }
        }
        unsafe {
            llvm_asm!("fence.i" :::: "volatile");
        }
        // map user stack
        let ustack = VMObjectPaged::new(page_count(USER_STACK_SIZE))?;
        let vsstack_pn = VirtAddr::from(elf_vaddr_end + PAGE_SIZE).ceil_page_num();
        aspace.map(ustack, 0, vsstack_pn, None, stack_pte_flags)?;
        info!("map user stack at {:#x?}", vsstack_pn.addr());
        Some(LoadedElf {
            entry,
            user_sp: usize::from(vsstack_pn.addr()) + USER_STACK_SIZE,
        })
    }
}

pub struct LoadedElf {
    pub entry: usize,
    pub user_sp: usize,
}
