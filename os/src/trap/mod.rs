pub mod context;

use crate::mm::addr::*;
use crate::mm::aspace::TRAMPOLINE_BASE_VPN;
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
    set_user_trap_entry()
}

#[allow(dead_code)]
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE_BASE_VPN.addr().into(), TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE_BASE_VPN.addr().into(), TrapMode::Direct);
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
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
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

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("Trap from kernel!");
}
