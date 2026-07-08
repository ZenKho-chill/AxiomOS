ENTRY(_start)

SECTIONS
{
    /* Định vị chương trình tại địa chỉ ảo 0x400000 theo đúng ADR-007 */
    . = 0x400000;

    .text :
    {
        *(.text .text.*)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }

    .data :
    {
        *(.data .data.*)
    }

    .bss :
    {
        *(.bss .bss.*)
    }
}
