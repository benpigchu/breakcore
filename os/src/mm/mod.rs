pub mod addr;
pub mod aspace;
mod frame;
mod page_table;
pub mod vmo;

pub fn init() {
    frame::init();
    aspace::kernel_aspace_init();
}

pub use page_table::PTEFlags;
