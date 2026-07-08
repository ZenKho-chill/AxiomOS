ENTRY(_start)

SECTIONS
{
    /* Định vị chương trình tại địa chỉ ảo 0x400000 theo đúng ADR-007 */
    . = 0x400000;

    .text :
    {
        *(.text .text.*)
    }

    . = ALIGN(0x1000);

    .rodata :
    {
        *(.rodata .rodata.*)
    }

    . = ALIGN(0x1000);

    .data :
    {
        *(.data .data.*)
    }

    .bss :
    {
        *(.bss .bss.*)
    }
}
