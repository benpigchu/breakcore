# 第一章：在 RV64 裸机上运行程序

## 开发环境

笔者是在 WSL 下的 Ubuntu 20.4 下进行开发的。在这个版本的 Ubuntu 中，可以直接使用 apt 安装较新版本的 qemu。在构建工具上我们不使用 make，而是使用 [cargo-make](https://github.com/sagiegurari/cargo-make)。

## 在 RV64 指令集 no_std 环境下下编译通过

这部分可以参考 [这里的描述](https://docs.rust-embedded.org/embedonomicon/smallest-no-std.html)。

首先我们在 `.cargo` 文件中指定默认 target 为 `riscv64gc-unknown-none-elf`，其次我们在 `os/src/main.rs` 中添加 `#![no_std]` 指明我们不需要使用 `std` （事实上这个 target 也没有 `std` 能让你使用）。

这时，由于标准的 `main` 函数需要一个在运行前先初始化运行时，所以还是不能编译通过。我们需要加上 `#![no_main]` 指明我们没有传统意义上的主函数。

最后，我们需要一个 `#[panic_handler]` 来指定程序 panic 时的行为。

完成以上内容之后就可以正确编译了，但是我们还不能加载运行它，而且其实程序此时也没有任何功能。

## 调整内存布局，初始化运行时，加载运行

要在 qemu 上加载运行我们的系统，我们需要使用一个 bootloader。这里我们使用 RustSBI。

首先我们需要让 qemu 加载 RustSBI。为此我们需要 [下载并解压RustSBI](https://github.com/luojia65/rustsbi/releases/tag/v0.0.2)，然后用 qemu 的 `-bios` 参数让其加载 RustSBI。为了方便，我们将这些操作写入 `Makefile.toml`，于是用 `cargo make run` 运行，可以看到：

```
[rustsbi] Version 0.1.0
.______       __    __      _______.___________.  _______..______   __
|   _  \     |  |  |  |    /       |           | /       ||   _  \ |  |
|  |_)  |    |  |  |  |   |   (----`---|  |----`|   (----`|  |_)  ||  |
|      /     |  |  |  |    \   \       |  |      \   \    |   _  < |  |
|  |\  \----.|  `--'  |.----)   |      |  |  .----)   |   |  |_)  ||  |
| _| `._____| \______/ |_______/       |__|  |_______/    |______/ |__|

[rustsbi] Platform: QEMU
[rustsbi] misa: RV64ACDFIMSU
[rustsbi] mideleg: 0x222
[rustsbi] medeleg: 0xb109
[rustsbi] Kernel entry: 0x80200000
panicked at 'invalid instruction, mepc: 0000000080200000, instruction: 0000000000000000', platform\qemu\src\main.rs:392:17
QEMU: Terminated
```

RustSBI 在 qemu 里跑起来了，但是我们的程序还没有加载进去，所以我们可以看到有 RustSBI 在试图加载程序并运行之后报了非法指令的错误。

接下来我们开始用 RustSBI 加载我们的内核。从上面的输出信息可以看出，RustSBI 加载程序后，被加载的程序会从 0x80200000 开始运行，所以我们需要自定义 linker 脚本来改写程序的内存排布。

首先我们需要保证 entry 一定在 kernel 的开头。为此我们包含一个 `entry.asm` 文件，在其中定义 `.text.entry` 段，并用 `global_asm!` 宏将其嵌入进去 ，这样我们就可以在 linker 脚本中将其放在最前。

其次我们需要自己分配栈空间。我们可以同样在 `entry.asm` 中分配栈空间，并在 entry 中使用它，并将其定义为 `.bss.stack` 段

最后，为了让内核能够获取到自身内存空间的各种信息，我们需要在  `entry.asm` 和 linker 脚本中设置各种符号，这样我们就可以通过 extern 拿到这些符号的地址。

那么这时，我们只需要在 entry 中调用我们的 rust 主函数，便可以进入我们的内核了。注意 rust 主函数需要有 `#[no_mangle]` 来避免命名修饰，以便汇编代码找到我们的主函数。

然而，我们编译出的 ELF 文件还不能直接扔进 qemu 里运行，因为 ELF 文件中还有很多元信息。因此我们需要使用 cargo-binutils 带的一些二进制工具来去除这些部分，得到一个纯净的内核镜像。