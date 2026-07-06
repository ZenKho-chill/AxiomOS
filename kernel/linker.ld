ENTRY(_start)

SECTIONS
{
    /* Limine khuyên dùng địa chỉ ảo cao này cho kernel chạy ở Long Mode */
    . = 0xffffffff80100000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
    }
}
