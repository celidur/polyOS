section .text
global review_fpu_operation
; int operation(input, output, syscall_number, arg1, arg2).
; Keep x87, its control word, and XMM0 live across a syscall or timer preemption.
review_fpu_operation:
    push ebp
    mov ebp, esp
    push ebx
    push esi
    push edi
    sub esp, 4
    fnstcw [ebp-16]
    mov esi, [ebp+8]
    mov edi, [ebp+12]
    fld qword [esi]
    fldcw [esi+24]
    movups xmm0, [esi+8]
    mov eax, [ebp+16]
    test eax, eax
    jnz .syscall
    mov ecx, 2000000
.spin:
    dec ecx
    jnz .spin
    jmp .done
.syscall:
    mov ebx, [ebp+20]
    mov ecx, [ebp+24]
    int 0x80
.done:
    fstp qword [edi]
    movups [edi+8], xmm0
    fnstcw [edi+24]
    fldcw [ebp-16]
    add esp, 4
    pop edi
    pop esi
    pop ebx
    pop ebp
    ret

global review_fpu_clobber
review_fpu_clobber:
    fninit
    pxor xmm0, xmm0
    ret

global review_fpu_control
review_fpu_control:
    sub esp, 4
    fnstcw [esp]
    movzx eax, word [esp]
    add esp, 4
    ret
