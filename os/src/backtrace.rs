pub fn print_backtrace() {
    extern "C" {
        fn stext();
        fn etext();
    }
    let stext = stext as usize;
    let etext = etext as usize;
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
    println!("text: {:#x?}-{:#x?}", stext, etext);
    let mut layer = 0usize;
    while ra < etext && ra >= stext && fp != 0x0 {
        println!("{:?}: {:#x?}", layer, ra);
        println!("    fp: {:#x?}", fp);
        unsafe {
            ra = (fp as *mut usize).offset(-1).read_volatile();
            fp = (fp as *mut usize).offset(-2).read_volatile();
        }
        layer += 1;
    }
}
