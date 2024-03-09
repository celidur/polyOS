#include <stdint.h>
#include "vga.h"
#include "terminal/terminal.h"
#include "io/io.h"
#include "keyboard/keyboard.h"
#include "font.h"
#include "memory/memory.h"

#define	VGA_AC_INDEX		0x3C0
#define	VGA_AC_WRITE		0x3C0
#define	VGA_AC_READ	     	0x3C1
#define	VGA_MISC_WRITE		0x3C2
#define VGA_SEQ_INDEX		0x3C4
#define VGA_SEQ_DATA		0x3C5
#define	VGA_DAC_READ_INDEX	0x3C7
#define	VGA_DAC_WRITE_INDEX	0x3C8
#define	VGA_DAC_DATA		0x3C9
#define	VGA_MISC_READ		0x3CC
#define VGA_GC_INDEX 		0x3CE
#define VGA_GC_DATA 		0x3CF
/*			COLOR emulation		MONO emulation */
#define VGA_CRTC_INDEX		0x3D4		/* 0x3B4 */
#define VGA_CRTC_DATA		0x3D5		/* 0x3B5 */
#define	VGA_INSTAT_READ		0x3DA

#define	VGA_NUM_SEQ_REGS	5
#define	VGA_NUM_CRTC_REGS	25
#define	VGA_NUM_GC_REGS		9
#define	VGA_NUM_AC_REGS		21
#define	VGA_NUM_REGS		(1 + VGA_NUM_SEQ_REGS + VGA_NUM_CRTC_REGS + \
				VGA_NUM_GC_REGS + VGA_NUM_AC_REGS)



/*****************************************************************************
VGA REGISTER DUMPS FOR VARIOUS TEXT MODES
******************************************************************************
()=to do
	40x25	(40x30)	40x50	(40x60)
	(45x25)	(45x30)	(45x50)	(45x60)
	80x25	(80x30)	80x50	(80x60)
	(90x25)	90x30	(90x50)	90x60
*****************************************************************************/
static uint8_t g_40x25_text[] =
{
/* MISC */
	0x67,
/* SEQ */
	0x03, 0x08, 0x03, 0x00, 0x02,
/* CRTC */
	0x2D, 0x27, 0x28, 0x90, 0x2B, 0xA0, 0xBF, 0x1F,
	0x00, 0x4F, 0x0D, 0x0E, 0x00, 0x00, 0x00, 0xA0,
	0x9C, 0x8E, 0x8F, 0x14, 0x1F, 0x96, 0xB9, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00,
};

static uint8_t g_40x50_text[] =
{
/* MISC */
	0x67,
/* SEQ */
	0x03, 0x08, 0x03, 0x00, 0x02,
/* CRTC */
	0x2D, 0x27, 0x28, 0x90, 0x2B, 0xA0, 0xBF, 0x1F,
	0x00, 0x47, 0x06, 0x07, 0x00, 0x00, 0x04, 0x60,
	0x9C, 0x8E, 0x8F, 0x14, 0x1F, 0x96, 0xB9, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00,
};

static uint8_t g_80x25_text[] =
{
/* MISC */
	0x67,
/* SEQ */
	0x03, 0x00, 0x03, 0x00, 0x02,
/* CRTC */
	0x5F, 0x4F, 0x50, 0x82, 0x55, 0x81, 0xBF, 0x1F,
	0x00, 0x4F, 0x0D, 0x0E, 0x00, 0x00, 0x00, 0x50,
	0x9C, 0x0E, 0x8F, 0x28, 0x1F, 0x96, 0xB9, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00
};

static uint8_t g_80x50_text[] =
{
/* MISC */
	0x67,
/* SEQ */
	0x03, 0x00, 0x03, 0x00, 0x02,
/* CRTC */
	0x5F, 0x4F, 0x50, 0x82, 0x55, 0x81, 0xBF, 0x1F,
	0x00, 0x47, 0x06, 0x07, 0x00, 0x00, 0x01, 0x40,
	0x9C, 0x8E, 0x8F, 0x28, 0x1F, 0x96, 0xB9, 0xA3,
	0xFF, 
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF, 
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00,
};

static uint8_t g_90x30_text[] =
{
/* MISC */
	0xE7,
/* SEQ */
	0x03, 0x01, 0x03, 0x00, 0x02,
/* CRTC */
	0x6B, 0x59, 0x5A, 0x82, 0x60, 0x8D, 0x0B, 0x3E,
	0x00, 0x4F, 0x0D, 0x0E, 0x00, 0x00, 0x00, 0x00,
	0xEA, 0x0C, 0xDF, 0x2D, 0x10, 0xE8, 0x05, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00,
};

static uint8_t g_90x60_text[] =
{
/* MISC */
	0xE7,
/* SEQ */
	0x03, 0x01, 0x03, 0x00, 0x02,
/* CRTC */
	0x6B, 0x59, 0x5A, 0x82, 0x60, 0x8D, 0x0B, 0x3E,
	0x00, 0x47, 0x06, 0x07, 0x00, 0x00, 0x00, 0x00,
	0xEA, 0x0C, 0xDF, 0x2D, 0x08, 0xE8, 0x05, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
	0x0C, 0x00, 0x0F, 0x08, 0x00,
};
/*****************************************************************************
VGA REGISTER DUMPS FOR VARIOUS GRAPHICS MODES
*****************************************************************************/
// static uint8_t g_640x480x2[] =
// {
// /* MISC */
// 	0xE3,
// /* SEQ */
// 	0x03, 0x01, 0x0F, 0x00, 0x06,
// /* CRTC */
// 	0x5F, 0x4F, 0x50, 0x82, 0x54, 0x80, 0x0B, 0x3E,
// 	0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
// 	0xEA, 0x0C, 0xDF, 0x28, 0x00, 0xE7, 0x04, 0xE3,
// 	0xFF,
// /* GC */
// 	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0F,
// 	0xFF,
// /* AC */
// 	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
// 	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
// 	0x01, 0x00, 0x0F, 0x00, 0x00
// };
/*****************************************************************************
*** NOTE: the mode described by g_320x200x4[]
is different from BIOS mode 05h in two ways:
- Framebuffer is at A000:0000 instead of B800:0000
- Framebuffer is linear (no screwy line-by-line CGA addressing)
*****************************************************************************/
static uint8_t g_320x200x4[] =
{
/* MISC */
	0x63,
/* SEQ */
	0x03, 0x09, 0x03, 0x00, 0x02,
/* CRTC */
	0x2D, 0x27, 0x28, 0x90, 0x2B, 0x80, 0xBF, 0x1F,
	0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x9C, 0x0E, 0x8F, 0x14, 0x00, 0x96, 0xB9, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x02, 0x00,
	0xFF,
/* AC */
	0x00, 0x13, 0x15, 0x17, 0x02, 0x04, 0x06, 0x07,
	0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
	0x01, 0x00, 0x03, 0x00, 0x00
};

// static uint8_t g_640x480x16[] =
// {
// /* MISC */
// 	0xE3,
// /* SEQ */
// 	0x03, 0x01, 0x08, 0x00, 0x06,
// /* CRTC */
// 	0x5F, 0x4F, 0x50, 0x82, 0x54, 0x80, 0x0B, 0x3E,
// 	0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
// 	0xEA, 0x0C, 0xDF, 0x28, 0x00, 0xE7, 0x04, 0xE3,
// 	0xFF,
// /* GC */
// 	0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x05, 0x0F,
// 	0xFF,
// /* AC */
// 	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x14, 0x07,
// 	0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
// 	0x01, 0x00, 0x0F, 0x00, 0x00
// };

// static uint8_t g_720x480x16[] =
// {
// /* MISC */
// 	0xE7,
// /* SEQ */
// 	0x03, 0x01, 0x08, 0x00, 0x06,
// /* CRTC */
// 	0x6B, 0x59, 0x5A, 0x82, 0x60, 0x8D, 0x0B, 0x3E,
// 	0x00, 0x40, 0x06, 0x07, 0x00, 0x00, 0x00, 0x00,
// 	0xEA, 0x0C, 0xDF, 0x2D, 0x08, 0xE8, 0x05, 0xE3,
// 	0xFF,
// /* GC */
// 	0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x05, 0x0F,
// 	0xFF,
// /* AC */
// 	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
// 	0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
// 	0x01, 0x00, 0x0F, 0x00, 0x00,
// };

static uint8_t g_320x200x256[] =
{
/* MISC */
	0x63,
/* SEQ */
	0x03, 0x01, 0x0F, 0x00, 0x0E,
/* CRTC */
	0x5F, 0x4F, 0x50, 0x82, 0x54, 0x80, 0xBF, 0x1F,
	0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x9C, 0x0E, 0x8F, 0x28,	0x40, 0x96, 0xB9, 0xA3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x05, 0x0F,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
	0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
	0x41, 0x00, 0x0F, 0x00,	0x00
};

static uint8_t g_320x200x256_modex[] =
{
/* MISC */
	0x63,
/* SEQ */
	0x03, 0x01, 0x0F, 0x00, 0x06,
/* CRTC */
	0x5F, 0x4F, 0x50, 0x82, 0x54, 0x80, 0xBF, 0x1F,
	0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x9C, 0x0E, 0x8F, 0x28, 0x00, 0x96, 0xB9, 0xE3,
	0xFF,
/* GC */
	0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x05, 0x0F,
	0xFF,
/* AC */
	0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
	0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
	0x41, 0x00, 0x0F, 0x00, 0x00
};

/* g_360x480x256_modex - to do */

/*
 * dump_state
 *	- dump VGA register state to the serial port
*/

static void dump(uint8_t *regs, uint32_t count)
{
	uint32_t i;

	i = 0;
	serial_printf("\t");
	for(; count != 0; count--)
	{
		serial_printf("0x%x,", *regs);
		i++;
		if(i >= 8)
		{
			i = 0;
			serial_printf("\n\t");
		}
		else
			serial_printf(" ");
		regs++;
	}
	serial_printf("\n");
}


static void dump_regs(uint8_t *regs)
{
	serial_printf("uint8_t g_mode[] =\n");
	serial_printf("{\n");
	/* dump MISCELLANEOUS reg */
	serial_printf("/* MISC */\n");
	serial_printf("\t0x%x,\n", *regs);
	regs++;
	/* dump SEQUENCER regs */
	serial_printf("/* SEQ */\n");
	dump(regs, VGA_NUM_SEQ_REGS);
	regs += VGA_NUM_SEQ_REGS;
	/* dump CRTC regs */
	serial_printf("/* CRTC */\n");
	dump(regs, VGA_NUM_CRTC_REGS);
	regs += VGA_NUM_CRTC_REGS;
	/* dump GRAPHICS CONTROLLER regs */
	serial_printf("/* GC */\n");
	dump(regs, VGA_NUM_GC_REGS);
	regs += VGA_NUM_GC_REGS;
	/* dump ATTRIBUTE CONTROLLER regs */
	serial_printf("/* AC */\n");
	dump(regs, VGA_NUM_AC_REGS);
	regs += VGA_NUM_AC_REGS;
	serial_printf("};\n");
}

static void read_regs(uint8_t *regs)
{
	uint32_t i;

	/* read MISCELLANEOUS reg */
	*regs = insb(VGA_MISC_READ);
	regs++;
	/* read SEQUENCER regs */
	for(i = 0; i < VGA_NUM_SEQ_REGS; i++)
	{
		outb(VGA_SEQ_INDEX, i);
		*regs = insb(VGA_SEQ_DATA);
		regs++;
	}
	/* read CRTC regs */
	for(i = 0; i < VGA_NUM_CRTC_REGS; i++)
	{
		outb(VGA_CRTC_INDEX, i);
		*regs = insb(VGA_CRTC_DATA);
		regs++;
	}
	/* read GRAPHICS CONTROLLER regs */
	for(i = 0; i < VGA_NUM_GC_REGS; i++)
	{
		outb(VGA_GC_INDEX, i);
		*regs = insb(VGA_GC_DATA);
		regs++;
	}
	/* read ATTRIBUTE CONTROLLER regs */
	for(i = 0; i < VGA_NUM_AC_REGS; i++)
	{
		(void)insb(VGA_INSTAT_READ);
		outb(VGA_AC_INDEX, i);
		*regs = insb(VGA_AC_READ);
		regs++;
	}
	/* lock 16-color palette and unblank display */
	(void)insb(VGA_INSTAT_READ);
	outb(VGA_AC_INDEX, 0x20);
}

void dump_state(void)
{
	uint8_t state[VGA_NUM_REGS];

	read_regs(state);
	dump_regs(state);
}

static void write_regs(uint8_t *regs)
{
	uint32_t i;

	/* write MISCELLANEOUS reg */
	outb(VGA_MISC_WRITE, *regs);
	regs++;
	/* write SEQUENCER regs */
	for(i = 0; i < VGA_NUM_SEQ_REGS; i++)
	{
		outb(VGA_SEQ_INDEX, i);
		outb(VGA_SEQ_DATA, *regs);
		regs++;
	}
	/* unlock CRTC registers */
	outb(VGA_CRTC_INDEX, 0x03);
	outb(VGA_CRTC_DATA, insb(VGA_CRTC_DATA) | 0x80);
	outb(VGA_CRTC_INDEX, 0x11);
	outb(VGA_CRTC_DATA, insb(VGA_CRTC_DATA) & ~0x80);
	/* make sure they remain unlocked */
	regs[0x03] |= 0x80;
	regs[0x11] &= ~0x80;
	/* write CRTC regs */
	for(i = 0; i < VGA_NUM_CRTC_REGS; i++)
	{
		outb(VGA_CRTC_INDEX, i);
		outb(VGA_CRTC_DATA, *regs);
		regs++;
	}
	/* write GRAPHICS CONTROLLER regs */
	for(i = 0; i < VGA_NUM_GC_REGS; i++)
	{
		outb(VGA_GC_INDEX, i);
		outb(VGA_GC_DATA, *regs);
		regs++;
	}
	/* write ATTRIBUTE CONTROLLER regs */
	for(i = 0; i < VGA_NUM_AC_REGS; i++)
	{
		(void)insb(VGA_INSTAT_READ);
		outb(VGA_AC_INDEX, i);
		outb(VGA_AC_WRITE, *regs);
		regs++;
	}
	/* lock 16-color palette and unblank display */
	(void)insb(VGA_INSTAT_READ);
	outb(VGA_AC_INDEX, 0x20);
}

static uint32_t get_fb_seg(void)
{
	outb(VGA_GC_INDEX, 6);
	uint32_t seg = (insb(VGA_GC_DATA) >> 2) & 3;
	switch(seg)
	{
	case 0:
	case 1:
		return 0xA000;
	case 2:
		return 0xB000;
	case 3:
		return 0xB800;
	}
	return seg;
}

/*****************************************************************************
*****************************************************************************/
static void pokeb(uint32_t seg, uint32_t off, uint8_t val)
{
	*(uint8_t*)((seg << 4) + off) = val;
}

static void pokew(uint32_t seg, uint32_t off, uint16_t val)
{
	*(uint16_t*)((seg << 4) + off) = val;
}

static void vmemwr(uint16_t dst_off, uint8_t *src, uint32_t count)
{
	memcpy((void*)((get_fb_seg() << 4) + dst_off), src, count);
}

static void vpokeb(uint32_t off, uint8_t val)
{
	pokeb(get_fb_seg(), off, val);
}

static uint32_t vpeekb(uint32_t off)
{
	return (get_fb_seg() << 4) + off;
}

/*****************************************************************************
*****************************************************************************/
static void (*g_write_pixel)(uint32_t x, uint32_t y, uint32_t c);
static uint32_t g_wd, g_ht;

static void set_plane(uint32_t p)
{
	p &= 3;
	uint8_t pmask = 1 << p;
	/* set read plane */
	outb(VGA_GC_INDEX, 4);
	outb(VGA_GC_DATA, p);
	/* set write plane */
	outb(VGA_SEQ_INDEX, 2);
	outb(VGA_SEQ_DATA, pmask);
}

static void write_pixel1(uint32_t x, uint32_t y, uint32_t c)
{
	uint32_t wd_in_bytes;
	uint32_t off, mask;

	c = (c & 1) * 0xFF;
	wd_in_bytes = g_wd / 8;
	off = wd_in_bytes * y + x / 8;
	x = (x & 7) * 1;
	mask = 0x80 >> x;
	vpokeb(off, (vpeekb(off) & ~mask) | (c & mask));
}

static void write_pixel2(uint32_t x, uint32_t y, uint32_t c)
{
	uint32_t wd_in_bytes, off, mask;

	c = (c & 3) * 0x55;
	wd_in_bytes = g_wd / 4;
	off = wd_in_bytes * y + x / 4;
	x = (x & 3) * 2;
	mask = 0xC0 >> x;
	vpokeb(off, (vpeekb(off) & ~mask) | (c & mask));
}

static void write_pixel4p(uint32_t x, uint32_t y, uint32_t c)
{
	uint32_t wd_in_bytes, off, mask, p, pmask;

	wd_in_bytes = g_wd / 8;
	off = wd_in_bytes * y + x / 8;
	x = (x & 7) * 1;
	mask = 0x80 >> x;
	pmask = 1;
	for(p = 0; p < 4; p++)
	{
		set_plane(p);
		if(pmask & c)
			vpokeb(off, vpeekb(off) | mask);
		else
			vpokeb(off, vpeekb(off) & ~mask);
		pmask <<= 1;
	}
}

static void write_pixel8(uint32_t x, uint32_t y, uint32_t c)
{
	uint32_t wd_in_bytes;
	uint32_t off;

	wd_in_bytes = g_wd;
	off = wd_in_bytes * y + x;
	vpokeb(off, c);
}

static void write_pixel8x(uint32_t x, uint32_t y, uint32_t c)
{
	uint32_t wd_in_bytes;
	uint32_t off;

	wd_in_bytes = g_wd / 4;
	off = wd_in_bytes * y + x / 4;
	set_plane(x & 3);
	vpokeb(off, c);
}

// /*****************************************************************************
// write font to plane P4 (assuming planes are named P1, P2, P4, P8)
// *****************************************************************************/
static void write_font(uint8_t *buf, uint32_t font_height)
{
	uint8_t seq2, seq4, gc4, gc5, gc6;
	uint32_t i;

	/* save registers
	set_plane() modifies GC 4 and SEQ 2, so save them as well */
	outb(VGA_SEQ_INDEX, 2);
	seq2 = insb(VGA_SEQ_DATA);

	outb(VGA_SEQ_INDEX, 4);
	seq4 = insb(VGA_SEQ_DATA);
	/* turn off even-odd addressing (set flat addressing)
	assume: chain-4 addressing already off */
	outb(VGA_SEQ_DATA, seq4 | 0x04);

	outb(VGA_GC_INDEX, 4);
	gc4 = insb(VGA_GC_DATA);

	outb(VGA_GC_INDEX, 5);
	gc5 = insb(VGA_GC_DATA);
	/* turn off even-odd addressing */
	outb(VGA_GC_DATA, gc5 & ~0x10);

	outb(VGA_GC_INDEX, 6);
	gc6 = insb(VGA_GC_DATA);
	/* turn off even-odd addressing */
	outb(VGA_GC_DATA, gc6 & ~0x02);
	/* write font to plane P4 */
	set_plane(2);
	/* write font 0 */
	for(i = 0; i < 256; i++)
	{
		vmemwr(16384u * 0 + i * 32, buf, font_height);
		buf += font_height;
	}
#if 0
/* write font 1 */
	for(i = 0; i < 256; i++)
	{
		vmemwr(16384u * 1 + i * 32, buf, font_height);
		buf += font_height;
	}
#endif
	/* restore registers */
	outb(VGA_SEQ_INDEX, 2);
	outb(VGA_SEQ_DATA, seq2);
	outb(VGA_SEQ_INDEX, 4);
	outb(VGA_SEQ_DATA, seq4);
	outb(VGA_GC_INDEX, 4);
	outb(VGA_GC_DATA, gc4);
	outb(VGA_GC_INDEX, 5);
	outb(VGA_GC_DATA, gc5);
	outb(VGA_GC_INDEX, 6);
	outb(VGA_GC_DATA, gc6);
}


/*****************************************************************************
*****************************************************************************/
static void draw_x(void)
{
	uint32_t x, y;

	/* clear screen */
	for(y = 0; y < g_ht; y++)
		for(x = 0; x < g_wd; x++)
			g_write_pixel(x, y, 0);
	/* draw 2-color X */
	for(y = 0; y < g_ht; y++)
	{
		g_write_pixel((g_wd - g_ht) / 2 + y, y, 1);
		g_write_pixel((g_ht + g_wd) / 2 - y, y, 2);
	}
	
	for (size_t i = 0; i < 900000000; i++)
    {
        asm volatile("nop");
    }
}
/*****************************************************************************
READ AND DUMP VGA REGISTER VALUES FOR CURRENT VIDEO MODE
This is where g_40x25_text[], g_80x50_text[], etc. came from :)
*****************************************************************************/

/*****************************************************************************
SET TEXT MODES
*****************************************************************************/
void set_text_mode(enum screen_mode mode)
{
	uint32_t rows, cols, ht, i;

	switch (mode)
	{
	case VGA_40x25_TEXT:
		write_regs(g_40x25_text);
		cols = 40;
		rows = 25;
		ht = 8;
		break;
	case VGA_40x50_TEXT:
		write_regs(g_40x50_text);
		cols = 40;
		rows = 50;
		ht = 8;
		break;
	case VGA_80x25_TEXT:
		write_regs(g_80x25_text);
		cols = 80;
		rows = 25;
		ht = 16;
		break;
	case VGA_80x50_TEXT:
		write_regs(g_80x50_text);
		cols = 80;
		rows = 50;
		ht = 16;
		break;
	case VGA_90x30_TEXT:
		write_regs(g_90x30_text);
		cols = 90;
		rows = 30;
		ht = 16;
		break;
	case VGA_90x60_TEXT:
		write_regs(g_90x60_text);
		cols = 90;
		rows = 60;
		ht = 8;
		break;
	
	default:
		break;
	}
	/* set font */
	if(ht >= 16)
		write_font(get_font(VGA_FONT_8x16), 16);
	else
		write_font(get_font(VGA_FONT_8x8), 8);
	/* tell the BIOS what we've done, so BIOS text output works OK */
	pokew(0x40, 0x4A, cols);	/* columns on screen */
	pokew(0x40, 0x4C, cols * rows * 2); /* framebuffer size */
	pokew(0x40, 0x50, 0);		/* cursor pos'n */
	pokeb(0x40, 0x60, ht - 1);	/* cursor shape */
	pokeb(0x40, 0x61, ht - 2);
	pokeb(0x40, 0x84, rows - 1);	/* rows on screen - 1 */
	pokeb(0x40, 0x85, ht);		/* char height */
	/* set white-on-black attributes for all text */
	for(i = 0; i < cols * rows; i++)
		pokeb(0xB800, i * 2 + 1, 7);
}
/*****************************************************************************
DEMO GRAPHICS MODES
*****************************************************************************/
void demo_graphics(void)
{
	serial_printf("Screen-clear in 16-color mode will be VERY SLOW\n"
		"Press a key to continue\n");
	for (size_t i = 0; i < 100000000; i++)
    {
        asm volatile("nop");
	}		
	
	/* 4-color */
	write_regs(g_320x200x4);
	memset((void*)(get_fb_seg() << 4), 0, 320 * 200 / 4);
	g_wd = 320;
	g_ht = 200;
	g_write_pixel = write_pixel2;
	for (size_t i = 0; i < 100000000; i++)
    {
        asm volatile("nop");
    }
	draw_x();
	// /* 16-color */
	// write_regs(g_640x480x16);
	// memset((void*)(get_fb_seg() << 4), 0, 640 * 480 / 2);
	// g_wd = 640;
	// g_ht = 480;
	// g_write_pixel = write_pixel4p;
	// draw_x();
	/* 256-color */
	write_regs(g_320x200x256);
	memset((void*)(get_fb_seg() << 4), 0, 320 * 200);
	g_wd = 320;
	g_ht = 200;
	g_write_pixel = write_pixel8;
	draw_x();
	/* 256-color Mode-X */
	write_regs(g_320x200x256_modex);
	memset((void*)(get_fb_seg() << 4), 0, 320 * 200);
	g_wd = 320;
	g_ht = 200;
	g_write_pixel = write_pixel8x;
	draw_x();
	/* go back to 80x25 text mode */
	set_text_mode(VGA_90x60_TEXT);
	memset((void*)(get_fb_seg() << 4), 0, 90 * 60 * 2);
}
/*****************************************************************************
*****************************************************************************/
static uint8_t reverse_bits(uint8_t arg)
{
	uint8_t ret_val = 0;

	if(arg & 0x01)
		ret_val |= 0x80;
	if(arg & 0x02)
		ret_val |= 0x40;
	if(arg & 0x04)
		ret_val |= 0x20;
	if(arg & 0x08)
		ret_val |= 0x10;
	if(arg & 0x10)
		ret_val |= 0x08;
	if(arg & 0x20)
		ret_val |= 0x04;
	if(arg & 0x40)
		ret_val |= 0x02;
	if(arg & 0x80)
		ret_val |= 0x01;
	return ret_val;
}
/*****************************************************************************
512-CHARACTER FONT
*****************************************************************************/
static void font512(void)
{
/* Turbo C++ 1.0 seems to 'lose' any data declared 'static const' */
	/*static*/ const char msg1[] = "!txet sdrawkcaB";
	/*static*/ const char msg2[] = "?rorrim a toG";
/**/
	uint8_t seq2, seq4, gc4, gc5, gc6;
	uint32_t font_height, i, j;

/* start in 80x25 text mode */
	set_text_mode(VGA_80x25_TEXT);
/* code pasted in from write_font():
save registers
set_plane() modifies GC 4 and SEQ 2, so save them as well */
	outb(VGA_SEQ_INDEX, 2);
	seq2 = insb(VGA_SEQ_DATA);

	outb(VGA_SEQ_INDEX, 4);
	seq4 = insb(VGA_SEQ_DATA);
/* turn off even-odd addressing (set flat addressing)
assume: chain-4 addressing already off */
	outb(VGA_SEQ_DATA, seq4 | 0x04);

	outb(VGA_GC_INDEX, 4);
	gc4 = insb(VGA_GC_DATA);

	outb(VGA_GC_INDEX, 5);
	gc5 = insb(VGA_GC_DATA);
/* turn off even-odd addressing */
	outb(VGA_GC_DATA, gc5 & ~0x10);

	outb(VGA_GC_INDEX, 6);
	gc6 = insb(VGA_GC_DATA);
/* turn off even-odd addressing */
	outb(VGA_GC_DATA, gc6 & ~0x02);
/* write font to plane P4 */
	set_plane(2);
/* this is different from write_font():
use font 1 instead of font 0, and use it for BACKWARD text */
	font_height = 16;
	uint8_t *g_8x16_font = get_font(VGA_FONT_8x16);	
	for(i = 0; i < 256; i++)
	{
		for(j = 0; j < font_height; j++)
		{
			vpokeb(16384u * 1 + 32 * i + j,
				reverse_bits(
					g_8x16_font[font_height * i + j]));
		}
	}
/* restore registers */
	outb(VGA_SEQ_INDEX, 2);
	outb(VGA_SEQ_DATA, seq2);
	outb(VGA_SEQ_INDEX, 4);
	outb(VGA_SEQ_DATA, seq4);
	outb(VGA_GC_INDEX, 4);
	outb(VGA_GC_DATA, gc4);
	outb(VGA_GC_INDEX, 5);
	outb(VGA_GC_DATA, gc5);
	outb(VGA_GC_INDEX, 6);
	outb(VGA_GC_DATA, gc6);
/* now: sacrifice attribute bit b3 (foreground intense color)
use it to select characters 256-511 in the second font */
	outb(VGA_SEQ_INDEX, 3);
	outb(VGA_SEQ_DATA, 4);
/* xxx - maybe re-program 16-color palette here
so attribute bit b3 is no longer used for 'intense' */
	for(i = 0; i < sizeof(msg1); i++)
	{
		vpokeb((80 * 8  + 40 + i) * 2 + 0, msg1[i]);
/* set attribute bit b3 for backward font */
		vpokeb((80 * 8  + 40 + i) * 2 + 1, 0x0F);
	}
	for(i = 0; i < sizeof(msg2); i++)
	{
		vpokeb((80 * 16 + 40 + i) * 2 + 0, msg2[i]);
		vpokeb((80 * 16 + 40 + i) * 2 + 1, 0x0F);
	}
}
/*****************************************************************************
*****************************************************************************/
// int main(int arg_c, char *arg_v[])
// {
// 	//dump_state();
// 	//set_text_mode(arg_c > 1);
// 	//demo_graphics();
// 	font512();
// 	return 0;
// }
