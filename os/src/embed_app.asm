
    .align 4
    .section .data
    .global app_list
app_list:
    .quad 3
    
    .quad app_0_start
    .quad app_0_end
    

    .quad app_1_start
    .quad app_1_end
    

    .quad app_2_start
    .quad app_2_end
    

    .align 4
    .section .data
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "target/riscv64gc-unknown-none-elf/00hello_world.bin"
app_0_end:
    
    .align 4
    .section .data
    .global app_1_start
    .global app_1_end
app_1_start:
    .incbin "target/riscv64gc-unknown-none-elf/01store_fault.bin"
app_1_end:
    
    .align 4
    .section .data
    .global app_2_start
    .global app_2_end
app_2_start:
    .incbin "target/riscv64gc-unknown-none-elf/02power.bin"
app_2_end:
    