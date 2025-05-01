[BITS 16]
ORG 0x7C00

start:
    jmp short boot_main
    nop

; FAT16 Header
OEMIdentifier     db "PolyOS  "
BytesPerSector    dw 0x200
SectorsPerCluster db 0x80
ReservedSectors   dw 0xFFFF
FATCopies         db 0x02
RootDirEntries    dw 0x40
NumSectors        dw 0x00
MediaType         db 0xF8
SectorsPerFAT     dw 0x100
SectorsPerTrack   dw 0x20
NumHeads          dw 0x40
HiddenSectors     dd 0x00
SectorsBig        dd 0x773594

; Extended BPB 
DriveNumber       db 0x80
WinNTBit          db 0x00
Signature         db 0x29
VolumeID          dd 0xD105
VolumeIDString    db "POLYOS BOOT"
SystemIDString    db "FAT16   "

boot_main:
    jmp 0:step2

step2:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    ; Load GDT and jump to protected mode
    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp CODE_SEG:protected_mode_start

; === GDT ===
gdt_start:
gdt_null:
    dq 0

; offset 0x8
gdt_code:     ; CS SHOULD POINT TO THIS
    dw 0xffff ; Segment limit first 0-15 bits
    dw 0      ; Base first 0-15 bits
    db 0      ; Base 16-23 bits
    db 0x9a   ; Access byte
    db 11001111b ; High 4 bit flags and the low 4 bit flags
    db 0        ; Base 24-31 bits

; offset 0x10
gdt_data:      ; DS, SS, ES, FS, GS
    dw 0xffff ; Segment limit first 0-15 bits
    dw 0      ; Base first 0-15 bits
    db 0      ; Base 16-23 bits
    db 0x92   ; Access byte
    db 11001111b ; High 4 bit flags and the low 4 bit flags
    db 0        ; Base 24-31 bits

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

CODE_SEG equ gdt_code - gdt_start
DATA_SEG equ gdt_data - gdt_start

; === Protected Mode ===
[BITS 32]
protected_mode_start:
    mov ax, DATA_SEG
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Enable A20
    in al, 0x92
    or al, 2
    out 0x92, al

    mov esi, 10
    mov eax, 1
    mov edi, 0x0100000

read_loop:
    push esi
    push edi
    push eax

    mov ecx, 255
    call ata_lba_read

    pop eax
    pop edi
    pop esi

    add edi, 255 * 512
    add eax, 255

    dec esi

    jnz read_loop

    jmp CODE_SEG:0x0100000

ata_lba_read:
    mov ebx, eax
    shr eax, 24
    or eax, 0xE0
    mov dx, 0x1F6
    out dx, al
    mov eax, ecx
    mov dx, 0x1F2
    out dx, al
    mov eax, ebx
    mov dx, 0x1F3
    out dx, al
    mov dx, 0x1F4
    shr eax, 8
    out dx, al
    mov dx, 0x1F5
    shr eax, 8
    out dx, al
    mov dx, 0x1F7
    mov al, 0x20
    out dx, al

.read_sector:
    push ecx
.wait_read:
    mov dx, 0x1F7
    in al, dx
    test al, 8
    jz .wait_read
    mov ecx, 256
    mov dx, 0x1F0
    rep insw
    pop ecx
    loop .read_sector
    ret

times 510-($ - $$) db 0
dw 0xAA55
