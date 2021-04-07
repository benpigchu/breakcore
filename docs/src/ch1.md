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
[rustsbi] Kernel entry: 0x80020000
panicked at 'invalid instruction, mepc: 0000000080200000, instruction: 0000000000000000', platform\qemu\src\main.rs:392:17
QEMU: Terminated
```

RustSBI 在 qemu 里跑起来了，但是我们的程序还没有加载进去，所以我们可以看到有 RustSBI 在试图加载程序并运行之后报了非法指令的错误。

接下来我们开始用 RustSBI 加载我们的内核。从上面的输出信息可以看出，RustSBI 加载程序后，被加载的程序会从 0x80020000 开始运行，所以我们需要自定义 linker 脚本来改写程序的内存排布。

首先我们需要保证 entry 一定在 kernel 的开头。为此我们包含一个 `entry.asm` 文件，在其中定义 `.text.entry` 段，并用 `global_asm!` 宏将其嵌入进去 ，这样我们就可以在 linker 脚本中将其放在最前。

其次我们需要自己分配栈空间。我们可以同样在 `entry.asm` 中分配栈空间，并在 entry 中使用它，并将其定义为 `.bss.stack` 段

最后，为了让内核能够获取到自身内存空间的各种信息，我们需要在  `entry.asm` 和 linker 脚本中设置各种符号，这样我们就可以通过 extern 拿到这些符号的地址。

那么这时，我们只需要在 entry 中调用我们的 rust 主函数，便可以进入我们的内核了。注意 rust 主函数需要有 `#[no_mangle]` 来避免命名修饰，以便汇编代码找到我们的主函数。

然而，我们编译出的 ELF 文件还不能直接扔进 qemu 里运行，因为 ELF 文件中还有很多元信息。因此我们需要使用 cargo-binutils 带的一些二进制工具来去除这些部分，得到一个纯净的内核镜像。

## 开始运行真正的代码

在进入我们的主程序之前，我们还需要清空用于存储全局变量的 bss 段以初始化全局变量。好在我们之前已经标记好各段位置的符号，所以只需要拿到这一段的起止地址逐个字节清零即可。注意：为了避免指令重排带来的不一致，我们需要使用 `write_volatile`。

接下来我们需要想办法输出我们的 hello world。在 RV64 当中，bootloader 运行在 M 态，而我们的内核运行在 S 态，我们可以通过 `ecall` 指令来调用 RustSBI 提供的接口来完成一些操作，而这就包括了直接从串口输出字符。可以在 [这里](https://github.com/riscv/riscv-sbi-doc/blob/master/riscv-sbi.adoc) 看到 SBI 接口的详细信息。我们只需要把 `ecall` 包装成函数调用，然后在其上包装出串口写操作，最后由此重建 `print!` 和 `println!` 宏即可。

当然，为了方便调试，我们还要在里 panic handler 里输出相关信息。简单调用一下输出就好。

## Bonus：更详细的报错信息 backtrace

如果我们能在 Panic 时打印 backtrace 信息，那么我们就可以更方便地 debug 了。由于我们知道栈上保存了函数调用相关的信息，所以我们可以直接读取这些信息并打印。

要打印 backtrace，我们首先要获得 fp 和 pc 的值，为此我们需要插入一段内联汇编来完成这个任务。然后我们就可以读取 fp-4 指向的内存获得返回地址，并读取 fp-8 指向的内存获得上一层的 fp。如此迭代下去我们就能打印包含 fp 和 pc 的基本的 backtrace 信息了。

如果要获得更详细的信息，比如所在的函数和文件，那就需要读取二进制文件中的调试信息了。我们可以利用 [gimli](https://docs.rs/gimli/0.23.0/gimli/) 来完成这项工作，但是那就需要使用内核堆内存空间了。我们将在第四章引入全局分配器之后再加以改善。

## 彩色调试信息

为了能够区分不同重要程度的调试信息，我们引入 `log` 包来打印不同等级的 log。为此我们创建一个 Logger 结构，为其实现 `Log` trait，并在初始化时设置这个 logger，然后便可以使用 `log` 包中提供的各种宏来打印 log 了。具体的实现上，我们使用 ANSI 转义序列来输出颜色，并为不同等级指定不同的颜色，这样便能用颜色来区分 log 的等级。