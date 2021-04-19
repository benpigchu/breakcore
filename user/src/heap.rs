use crate::{console::print_no_heap, sys_exit, sys_mmap, MMapprot};
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 0x10000;
const USER_STACK_SIZE: usize = 4096 * 16;
const PAGE_SIZE: usize = 4096;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    print_no_heap(format_args!("OOM! layout = {:#x?}", layout));
    sys_exit(-1);
}

pub fn init() {
    unsafe {
        extern "C" {
            fn end_elf();
        }
        // Don't overlap with stack allocated by the kernel
        let start_heap = end_elf as usize + USER_STACK_SIZE + PAGE_SIZE * 2;
        let prot = MMapprot::READ | MMapprot::WRITE;
        if USER_HEAP_SIZE as isize != sys_mmap(start_heap, USER_HEAP_SIZE, prot) {
            print_no_heap(format_args!(
                "heap init dailed! start={:#x?} len={:#x?}",
                start_heap, USER_HEAP_SIZE
            ));
            sys_exit(-1);
        };
        HEAP_ALLOCATOR.lock().init(start_heap, USER_HEAP_SIZE);
    }
}
