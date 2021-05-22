use addr2line::gimli;
use addr2line::Context;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::option::Option;
use core::slice;
use lazy_static::*;
use log::*;
use object::{File, Object, ObjectSection};

pub fn print_backtrace() {
    print!("\u{1B}[31m");
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
        let mut frames = ADDR2LINE_CONTEXT
            .as_ref()
            .and_then(|ctx| ctx.context.find_frames(ra as u64).ok());
        if let Some(frames) = frames.as_mut() {
            while let Ok(Some(frame)) = frames.next() {
                print!("    ");
                if let Some(function) = frame.function {
                    let name = function.demangle();
                    print!(
                        "{} ",
                        name.as_ref()
                            .map_or("<Error getting name>", |name| name.as_ref())
                    );
                }
                if let Some(location) = frame.location {
                    print!("at {}", location.file.unwrap_or("<Error getting file>"));
                    if let Some(line) = location.line {
                        print!(":{}", line);
                        if let Some(column) = location.column {
                            print!(":{}", column);
                        }
                    }
                }
                #[allow(clippy::println_empty_string)]
                println!("")
            }
        }
        println!("    fp: {:#x?}", fp);
        unsafe {
            ra = (fp as *mut usize).offset(-1).read_volatile();
            fp = (fp as *mut usize).offset(-2).read_volatile();
        }
        layer += 1;
    }
    print!("\u{1B}[0m");
}

struct Addr2LineContext {
    context: Context<gimli::EndianSlice<'static, gimli::RunTimeEndian>>,
}

unsafe impl Sync for Addr2LineContext {}

fn load_debuginfo() -> Option<Addr2LineContext> {
    let debuginfo_range =
        unsafe { slice::from_raw_parts_mut(DEBUGINFO_ELF_ADDRESS as *mut u8, DEBUGINFO_ELF_SIZE) };
    let debuginfo_elf = File::parse(debuginfo_range).ok()?;
    let endian = if debuginfo_elf.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };
    let load_section = |id: gimli::SectionId| -> Result<_, gimli::Error> {
        match debuginfo_elf.section_by_name(id.name()).as_ref() {
            Some(section) => Ok(section
                .uncompressed_data()
                .unwrap_or(Cow::Borrowed(&[][..]))),
            None => Ok(Cow::Borrowed(&[][..])),
        }
    };
    let load_sep_section = |_| Ok(Cow::Borrowed(&[][..]));
    let dwarf_cow = Box::leak(Box::new(
        gimli::Dwarf::load(load_section, load_sep_section).ok()?,
    ));
    let borrow_section: &dyn for<'a> Fn(
        &'a Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);
    let dwarf = dwarf_cow.borrow(&borrow_section);
    addr2line::Context::from_dwarf(dwarf)
        .map(|context| Addr2LineContext { context })
        .ok()
}

pub const DEBUGINFO_ELF_ADDRESS: usize = 0x80800000;
pub const DEBUGINFO_ELF_SIZE: usize = 0x01800000;
lazy_static! {
    static ref ADDR2LINE_CONTEXT: Option<Addr2LineContext> = load_debuginfo();
}

pub fn init() {
    extern "C" {
        fn skernel();
        fn ekernel();
    }
    info!("kernel: {:#x?}-{:#x?}", skernel as usize, ekernel as usize);
    info!("addr2line ok? {:?}", ADDR2LINE_CONTEXT.is_some());
}
