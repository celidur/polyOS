bits 32
section .text
global syscall
extern __errno_location
; Like the GNU ABI entry, fetch raw argument slots in assembly. C va_arg on
; unspecified arguments would be undefined for calls such as syscall(gettid).
syscall:
    push ebp
    push ebx
    push esi
    push edi
    mov eax, [esp+20]
    mov ebx, [esp+24]
    mov ecx, [esp+28]
    mov edx, [esp+32]
    mov esi, [esp+36]
    mov edi, [esp+40]
    mov ebp, [esp+44]
    int 0x80
    pop edi
    pop esi
    pop ebx
    pop ebp
    cmp eax, -4095
    jb .done
    push eax
    sub esp, 8
    call __errno_location
    add esp, 8
    pop edx
    neg edx
    mov [eax], edx
    mov eax, -1
.done:
    ret
