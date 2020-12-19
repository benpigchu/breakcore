use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=../user/src/bin/");
    gen_embed_app_asm()
}
fn gen_embed_app_asm() {
    let app_names = ["00hello_world", "01store_fault", "02power"];
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("src/embed_app.asm")
        .unwrap();
    write!(
        f,
        r#"
    .align 4
    .section .data
    .global app_list
app_list:
    .quad {}
    "#,
        app_names.len()
    )
    .unwrap();
    for i in 0..app_names.len() {
        writeln!(
            f,
            r#"
    .quad app_{}_start
    .quad app_{}_end
    "#,
            i, i
        )
        .unwrap();
    }
    for (i, name) in app_names.iter().enumerate() {
        // We moved the binary when we run the makefile,
        // so we do not need to change the incbin path when changing profile
        let app_bin_path = format!("target/{}/{}.bin", env::var("TARGET").unwrap(), name);
        println!("cargo:rerun-if-changed=../user/src/bin/{}.rs", name);
        println!("cargo:rerun-if-changed={}", app_bin_path);

        write!(
            f,
            r#"
    .align 4
    .section .data
    .global app_{}_start
    .global app_{}_end
app_{}_start:
    .incbin "{}"
app_{}_end:
    "#,
            i, i, i, app_bin_path, i
        )
        .unwrap();
    }
}
