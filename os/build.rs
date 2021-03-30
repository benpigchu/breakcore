use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=../user/src/bin/");
    gen_embed_app_asm()
}
fn gen_embed_app_asm() {
    println!("cargo:rerun-if-env-changed=USER_PROGRAMS");
    let app_names_raw = env::var("USER_PROGRAMS").unwrap_or_else(|_| "".to_owned());
    let app_names: Vec<_> = app_names_raw.split_terminator(' ').collect();
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("src/embed_app.asm")
        .unwrap();
    write!(
        f,
        r#"
    .balign 16
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
        let app_bin_path = format!("target/{}/{}", env::var("TARGET").unwrap(), name);
        println!("cargo:rerun-if-changed=../user/src/bin/{}.rs", name);
        println!("cargo:rerun-if-changed=../{}", app_bin_path);

        write!(
            f,
            r#"
    .balign 16
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
