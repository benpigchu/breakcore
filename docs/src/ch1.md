# 第一章：在 RV64 裸机上运行程序

## 开发环境

笔者是在 WSL 下的 Ubuntu 20.4 下进行开发的。在这个版本的 Ubuntu 中，可以直接使用 apt 安装较新版本的 qemu。在构建工具上我们不使用 make，而是使用 [cargo-make](https://github.com/sagiegurari/cargo-make)。

## 在 RV64 指令集 no_std 环境下下编译通过

首先我们在 `.cargo` 文件中指定默认 target 为 `riscv64gc-unknown-none-elf`，其次我们在 `os/src/main.rs` 中添加 `#![no_std]` 指明我们不需要使用 `std` （事实上这个 target 也没有 `std` 能让你使用）。

这时，由于标准的 `main` 函数需要一个在运行前先初始化运行时，所以还是不能编译通过。我们需要加上 `#![no_main]` 指明我们没有传统意义上的主函数。

最后，我们需要一个 `#[panic_handler]` 来指定程序 panic 时的行为。

完成以上内容之后就可以正确编译了，但是我们还不能加载运行它，而且其实程序此时也没有任何功能。

## 调整内存布局，初始化运行时，加载运行
