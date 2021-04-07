ENTRY(_start)
BASE_ADDRESS = 0x80020000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        etrampoline = .;
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        . += 0x1000;
        . = ALIGN(4K);
        sstack = .;
        *(.bss.stack)
        . = ALIGN(4K);
        estack = .;
        . += 0x1000;
        . = ALIGN(4K);
        sbss = .;
        *(.bss .bss.*)
    }
    .sbss : {
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}