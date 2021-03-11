pub mod addr;
mod aspace;
mod frame;
mod page_table;

pub fn init() {
    frame::init();
    aspace::kernel_aspace_init();
}
