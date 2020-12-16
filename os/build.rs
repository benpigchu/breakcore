use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=../user/src/bin/");
    gen_embed_app_asm()
}
fn gen_embed_app_asm() {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("src/embed_app.asm")
        .unwrap();
    // We moved the binary when we run the makefile,
    // so we do not need to change the incbin path when changing profile
    let app_bin_path = format!("target/{}/00hello_world.bin", env::var("TARGET").unwrap());
    write!(
        f,
        r#"
    .align 4
    .section .data
    .global app_start
    .global app_end
app_start:
    .incbin "{}"
app_end:
	"#,
        app_bin_path
    )
    .unwrap();
}
