use crate::mm::addr::*;
use crate::mm::aspace::{create_user_aspace, AddressSpace, KERNEL_ASPACE, TRAMPOLINE_BASE_VPN};
use crate::mm::vmo::{VMObject, VMObjectPaged};
use crate::mm::PTEFlags;
use crate::task::TaskContext;
use crate::trap::context::TrapContext;
use alloc::sync::Arc;
use core::slice;
use lazy_static::*;
use log::*;

global_asm!(include_str!("embed_app.asm"));

pub const USER_STACK_SIZE: usize = 4096 * 16;
pub const KERNEL_STACK_SIZE: usize = 4096 * 16;
pub const MAX_APP_NUM: usize = 16;
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, task_cx: TaskContext) -> usize {
        let task_cx_ptr =
            (self.get_sp() as usize - core::mem::size_of::<TaskContext>()) as *mut TaskContext;
        unsafe {
            *task_cx_ptr = task_cx;
        }
        task_cx_ptr as usize
    }
}

fn init_stack(kstack: &mut KernelStack) -> usize {
    kstack.push_context(TaskContext::goto_launch())
}

#[derive(Default)]
struct AppInfo {
    start: usize,
    end: usize,
    name: &'static str,
}

pub struct AppManager {
    pub app_num: usize,
    apps: [AppInfo; MAX_APP_NUM],
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
        AppManager { app_num, apps }
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
    pub fn load_elf(&self, id: usize, aspace: &Arc<AddressSpace>) -> LoadedElf {
        if id >= self.app_num {
            panic!("Out of range app id!")
        }
        let app = &self.apps[id];
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
        let file_header = FileHeader64::<LittleEndian>::parse(app_bin_bytes).unwrap();
        info!("Parsing ELF for {}...", app.name);
        assert!(file_header.is_little_endian());
        assert!(file_header.is_class_64());
        assert_eq!(file_header.e_machine(LittleEndian), EM_RISCV);
        assert_eq!(file_header.e_type(LittleEndian), ET_EXEC);
        let entry = file_header.e_entry(LittleEndian) as usize;
        let program_headers = file_header
            .program_headers(LittleEndian, app_bin_bytes)
            .unwrap();
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
                    let vmo = VMObjectPaged::new(page_count(mem_size)).unwrap();
                    let buf = program_header
                        .data_as_array(LittleEndian, app_bin_bytes)
                        .unwrap();
                    let wrote_size = vmo.write(0, buf);
                    assert_eq!(buf.len(), wrote_size);
                    aspace
                        .map(
                            vmo,
                            0,
                            VirtAddr::from(vaddr_start).floor_page_num(),
                            None,
                            pte_flags,
                        )
                        .unwrap();
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
                    panic!("Dynamic linking is not supported");
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
        let ustack = VMObjectPaged::new(page_count(USER_STACK_SIZE)).unwrap();
        let vsstack_pn = VirtAddr::from(elf_vaddr_end + PAGE_SIZE).ceil_page_num();
        aspace.map(ustack, 0, vsstack_pn, None, stack_pte_flags);
        info!("map user stack at {:#x?}", vsstack_pn.addr());
        LoadedElf {
            entry,
            user_sp: usize::from(vsstack_pn.addr()) + USER_STACK_SIZE,
        }
    }
    pub fn load_app(&self, id: usize) -> LoadedApp {
        // map kernel stack
        let kstack_vmo = VMObjectPaged::new(page_count(KERNEL_STACK_SIZE)).unwrap();
        let vskstack =
            usize::from(TRAMPOLINE_BASE_VPN.addr()) - (KERNEL_STACK_SIZE + PAGE_SIZE) * (id + 1);
        KERNEL_ASPACE.map(
            kstack_vmo,
            0,
            VirtAddr::from(vskstack).floor_page_num(),
            None,
            PTEFlags::R | PTEFlags::W,
        );
        let kstack = unsafe { (vskstack as *mut KernelStack).as_mut().unwrap() };
        let kernel_sp = init_stack(kstack);

        let (aspace, trap_cx_ptr) = create_user_aspace();

        let token = aspace.token();
        let trap_cx_ref = unsafe { (trap_cx_ptr as *mut TrapContext).as_mut() }.unwrap();
        let user_cx = TrapContext::new(token, kstack.get_sp(), trap_cx_ptr);
        *trap_cx_ref = user_cx;

        let loaded_elf = self.load_elf(id, &aspace);
        trap_cx_ref.set_pc(loaded_elf.entry);
        trap_cx_ref.set_sp(loaded_elf.user_sp);
        LoadedApp {
            aspace,
            kernel_sp,
            trap_cx_ptr,
        }
    }
}

pub struct LoadedApp {
    pub aspace: Arc<AddressSpace>,
    pub kernel_sp: usize,
    pub trap_cx_ptr: usize,
}

pub struct LoadedElf {
    pub entry: usize,
    pub user_sp: usize,
}
