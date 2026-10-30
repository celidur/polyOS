ENTRY(_start)
OUTPUT_FORMAT(binary)
SECTIONS
{
    . = 1M;
    .start : ALIGN(4096)
    {
        *(.start)
    }
    
    .text : ALIGN(4096)
    {
        *(.text)
    }

    .asm : ALIGN(4096)
    {
        *(.asm)
    }

    .rodata : ALIGN(4096)
    {
        *(.rodata)
    }

    .device_drivers : ALIGN(4)
    {
        __device_drivers_start = .;
        KEEP(*(.device_drivers))
        __device_drivers_end = .;
    }

    .device_nodes : ALIGN(4)
    {
        __device_nodes_start = .;
        KEEP(*(.device_nodes))
        __device_nodes_end = .;
    }

    .data : ALIGN(4096)
    {
        *(.data)
    }

    .bss : ALIGN(4096)
    {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }
    ASSERT(__bss_end < 0x001f0000, "kernel data overlaps boot stack")
}
