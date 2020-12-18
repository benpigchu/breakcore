pub mod context;

use context::TrapContext;
use riscv::register::{mtvec::TrapMode, stvec};

global_asm!(include_str!("trap.asm"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[no_mangle]
extern "C" fn trap_handler(cx: *mut TrapContext) -> *mut TrapContext {
    let cx = unsafe { cx.as_mut().unwrap() };
    println!("We are back to kernel!");
    println!("cx: {:#x?}", cx);
    loop {}
    cx as *mut _
}
