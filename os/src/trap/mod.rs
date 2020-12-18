pub mod context;

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
extern "C" fn trap_handler() -> ! {
    println!("We are back to kernel!");
    loop {}
}
