#ifndef TSS_H
#define TSS_H

#include <os/types.h>

struct tss
{
    u32 link;
    u32 esp0; // kernel stack pointer
    u32 ss0;  // kernel stack segment
    u32 esp1;
    u32 ss1;
    u32 esp2;
    u32 ss2;
    u32 sr3;
    u32 eip;
    u32 eflags;
    u32 eax;
    u32 ecx;
    u32 edx;
    u32 ebx;
    u32 esp;
    u32 ebp;
    u32 esi;
    u32 edi;
    u32 es;
    u32 cs;
    u32 ss;
    u32 ds;
    u32 fs;
    u32 gs;
    u32 ldtr;
    u32 iopb;
    u32 ssp;
} __attribute__((packed));

void tss_load(int tss_segment);
#endif