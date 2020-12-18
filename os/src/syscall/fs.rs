const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
    let str = unsafe { core::str::from_utf8_unchecked(slice) };
    match fd {
        FD_STDOUT => {
            print!("{}", str);
            len as isize
        }
        _ => -1,
    }
}
