/* kernel_qemu/linker.ld */
ENTRY(_start)

SECTIONS
{
	/* Bootloader loads at 1M */
	. = 1M;

	/* The multiboot header must be very early */
	.text BLOCK(4K) : ALIGN(4K)
	{
		*(.multiboot)
		*(.text)
	}

	/* Read-only data. */
	.rodata BLOCK(4K) : ALIGN(4K)
	{
		*(.rodata)
	}

	/* Read-write initialized data */
	.data BLOCK(4K) : ALIGN(4K)
	{
		*(.data)
	}

	/* BSS and stack */
	.bss BLOCK(4K) : ALIGN(4K)
	{
		*(COMMON)
		*(.bss)
	}
}
