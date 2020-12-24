use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-env-changed=USER_BASE_ADDRESS");
    gen_liner_script()
}
fn gen_liner_script() {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("src/linker.ld")
        .unwrap();
    write!(
        f,
        r#"
OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = {};

SECTIONS
{{
    . = BASE_ADDRESS;
    .text : {{
        *(.text.entry)
        *(.text .text.*)
    }}
    .rodata : {{
        *(.rodata .rodata.*)
    }}
    .data : {{
        *(.data .data.*)
    }}
    .bss : {{
        start_bss = .;
        *(.bss .bss.*)
        end_bss = .;
    }}
    /DISCARD/ : {{
        *(.eh_frame)
        *(.debug*)
    }}
}}
        "#,
        env::var("USER_BASE_ADDRESS").unwrap_or_else(|_| "0x80080000".to_owned())
    )
    .unwrap()
}
