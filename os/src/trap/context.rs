use riscv::register::sstatus::{self, Sstatus, SPP};

use super::trap_handler;
use crate::mm::addr::*;
use crate::mm::aspace::{KERNEL_ASPACE, USER_CX_BASE_VPN};
#[repr(C)]
#[derive(Debug)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub trap_handler_address: usize,
    pub user_satp: usize,
    pub kernel_satp: usize,
    pub user_cx_addr: usize,
    pub kernel_cx_addr: usize,
    pub kernel_sp: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn new(
        pc: usize,
        sp: usize,
        user_satp: usize,
        kernel_kstack: usize,
        kernel_cx_addr: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: pc,
            trap_handler_address: trap_handler as usize,
            user_satp,
            kernel_satp: KERNEL_ASPACE.token(),
            user_cx_addr: usize::from(USER_CX_BASE_VPN.addr()),
            kernel_cx_addr,
            kernel_sp: kernel_kstack,
        };
        cx.set_sp(sp);
        cx
    }
}
