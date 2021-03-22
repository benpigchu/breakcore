use riscv::register::sstatus::{self, Sstatus, SPP};

use super::trap_handler;
#[repr(C)]
#[derive(Debug)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub trap_handler_address: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn new(pc: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: pc,
            trap_handler_address: trap_handler as usize,
        };
        cx.set_sp(sp);
        cx
    }
}
