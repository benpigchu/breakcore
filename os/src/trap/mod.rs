pub mod context;

use crate::loader::exit_app;
use crate::syscall::syscall;
use context::TrapContext;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

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
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], cx.x[10], cx.x[11], cx.x[12]) as usize
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault) => {
            println!("[kernel] Page fault in application, stval = {:#x}", stval);
            exit_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!(
                "[kernel] Illegal instruction in application, stval = {:#x}",
                stval
            );
            exit_app();
        }
        cause => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", cause, stval);
        }
    }
    cx as *mut _
}
