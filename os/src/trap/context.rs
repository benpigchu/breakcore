#[repr(C)]
pub struct TrapContext {
    pub sp: usize,
    pub sepc: usize,
}

impl TrapContext {
    pub fn new(sepc: usize, sp: usize) -> Self {
        TrapContext { sp, sepc }
    }
}
