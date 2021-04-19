use crate::{console::print_no_heap, sys_exit, sys_mmap, MMapprot};
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 0x10000;

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
            fn start_heap();
        }
        let start_heap = start_heap as usize;
        let prot = MMapprot::READ | MMapprot::WRITE;
        if USER_HEAP_SIZE as isize != sys_mmap(start_heap, USER_HEAP_SIZE, prot) {
            print_no_heap(format_args!("heap init dailed!"));
            sys_exit(-1);
        };
        HEAP_ALLOCATOR.lock().init(start_heap, USER_HEAP_SIZE);
    }
}
