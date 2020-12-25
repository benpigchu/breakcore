use crate::loader::*;
use crate::sbi::shutdown;
use core::cell::RefCell;
use lazy_static::*;

pub struct TaskManager {
    app_num: usize,
    next_app_id: RefCell<usize>,
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = TaskManager {
        app_num: APP_MANAGER.app_num,
        next_app_id: RefCell::new(0),
    };
}

impl TaskManager {
    fn launch_app(&self, id: usize) -> ! {
        APP_MANAGER.load_app(id);
        extern "C" {
            fn __restore(kernel_sp: usize);
        }
        unsafe {
            __restore(init_stack(id));
        }
        unreachable!("We are already in user space!");
    }

    pub fn run_next_app(&self) -> ! {
        let mut next_app = self.next_app_id.borrow_mut();
        if *next_app >= self.app_num {
            println!("[kernel] No more app!");
            shutdown()
        } else {
            println!("[kernel] load app: {}", *next_app);
            let next = *next_app;
            *next_app += 1;
            drop(next_app);
            self.launch_app(next)
        }
    }

    pub fn exit_app(&self) -> ! {
        self.run_next_app()
    }
}
