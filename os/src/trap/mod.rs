pub mod context;

use crate::task::TASK_MANAGER;
use crate::{syscall::syscall, timer};
use context::TrapContext;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    sstatus, stval, stvec,
};

global_asm!(include_str!("trap.asm"));

pub fn init() {
    unsafe {
        sstatus::set_spie();
    }
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
            TASK_MANAGER.exit_task(-1);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!(
                "[kernel] Illegal instruction in application, stval = {:#x}",
                stval
            );
            TASK_MANAGER.exit_task(-1);
        }
        Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            timer::schedule_next();
            TASK_MANAGER.switch_task();
        }
        cause => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", cause, stval);
        }
    }
    cx as *mut _
}
