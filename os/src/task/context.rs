global_asm!(include_str!("switch.asm"));

extern "C" {
    pub fn __switch(current_kernel_sp_ptr: usize, next_kernel_sp_ptr: usize);
}

#[repr(C)]
pub struct TaskContext {
    ra: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn goto_restore() -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            s: [0; 12],
        }
    }
}
