pub fn print_backtrace() {
    extern "C" {
        fn boot_stack();
        fn boot_stack_top();
    }
    let stack_addr = boot_stack as usize;
    let stack_addr_top = boot_stack_top as usize;
    let mut fp: usize;
    let mut ra: usize;
    unsafe {
        llvm_asm!("mv $0,fp;auipc $1,0x0"
            : "=r" (fp),"=r" (ra)
            :
            :
            : "volatile"
        );
    }
    println!("stack: {:#x?}-{:#x?}", stack_addr, stack_addr_top);
    let mut layer = 0usize;
    loop {
        println!("{:?}: {:#x?}", layer, ra);
        println!("    fp: {:#x?}", fp);
        if fp > stack_addr_top || fp <= stack_addr {
            break;
        }
        unsafe {
            ra = (fp as *mut usize).offset(-1).read_volatile();
            fp = (fp as *mut usize).offset(-2).read_volatile();
        }
        layer += 1;
    }
}
