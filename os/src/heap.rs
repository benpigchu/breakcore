use crate::sbi::shutdown;
use buddy_system_allocator::LockedHeap;

const KERNEL_HEAP_SIZE: usize = 0x400000;
#[repr(align(4096))]
struct Heap {
    data: [u8; KERNEL_HEAP_SIZE],
}
static mut HEAP_SPACE: Heap = Heap {
    data: [0; KERNEL_HEAP_SIZE],
};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    println!("[kernel] OOM! layout = {:#x?}", layout);
    shutdown()
}

pub fn init() {
    unsafe {
        let heap_start = HEAP_SPACE.data.as_ptr() as usize;
        println!("[kernel] heap: {:#x?}+{:#x?}", heap_start, KERNEL_HEAP_SIZE);
        HEAP_ALLOCATOR.lock().init(heap_start, KERNEL_HEAP_SIZE);
    }
}

#[allow(dead_code)]
fn heap_test() -> ! {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    // test alloc/dealloc
    let boxed_usize = Box::new(5usize);
    assert_eq!(*boxed_usize, 5);
    let heap_start = unsafe { HEAP_SPACE.data.as_ptr() as usize };
    assert!((heap_start..(heap_start + KERNEL_HEAP_SIZE))
        .contains(&(boxed_usize.as_ref() as *const _ as usize)));
    drop(boxed_usize);
    // test realloc
    let mut small_vec = Vec::<usize>::with_capacity(0x10);
    for i in 0..0x100 {
        small_vec.push(i);
    }
    for (i, n) in small_vec.iter().enumerate() {
        assert_eq!(*n, i);
    }
    small_vec.truncate(0x10);
    small_vec.shrink_to_fit();
    for (i, n) in small_vec.iter().enumerate() {
        assert_eq!(*n, i);
    }
    println!("[kernel] Basic heap test passed");
    println!("[kernel] Next alloc should OOM");
    let _very_large_vec = Vec::<u8>::with_capacity(KERNEL_HEAP_SIZE + 1);
    unreachable!("Should OOM here!");
}
