pub mod addr;
mod frame;
mod page_table;

pub fn init() {
    frame::init()
}
