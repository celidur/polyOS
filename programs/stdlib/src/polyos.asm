[BITS 32]

section .asm

global print:function
global polyos_getkey:function
global polyos_putchar:function
global polyos_malloc:function
global polyos_free:function
global polyos_process_load_start:function
global polyos_system:function
global polyos_process_get_args:function
global polyos_exit:function

; void print(char *str)
print:
    push ebp
    mov ebp, esp
    push dword [ebp+8]
    mov eax, 1 ; Command print
    int 0x80
    add esp, 4
    pop ebp
    ret

; int polyos_getkey()
polyos_getkey:
    push ebp
    mov ebp, esp
    mov eax, 2 ; Command getkey
    int 0x80
    pop ebp
    ret

; void polyos_putchar(char c)
polyos_putchar:
    push ebp
    mov ebp, esp
    mov eax, 3 ; Command putchar
    push dword [ebp+8] ; c
    int 0x80
    add esp, 4
    pop ebp
    ret

; void* polyos_malloc(size_t size)
polyos_malloc:
    push ebp
    mov ebp, esp
    mov eax, 4 ; Command malloc
    push dword [ebp+8] ; size
    int 0x80
    add esp, 4
    pop ebp
    ret

; void polyos_free(void *ptr)
polyos_free:
    push ebp
    mov ebp, esp
    mov eax, 5 ; Command free
    push dword [ebp+8] ; ptr
    int 0x80
    add esp, 4
    pop ebp
    ret

; void polyos_process_load_start(const char *filename)
polyos_process_load_start:
    push ebp
    mov ebp, esp
    mov eax, 6 ; Command process_load_start
    push dword [ebp+8] ; filename
    int 0x80
    add esp, 4
    pop ebp
    ret

; int polyos_system(struct command_arguemnts *args)
polyos_system:
    push ebp
    mov ebp, esp
    mov eax, 7 ; Command system
    push dword [ebp+8] ; args
    int 0x80
    add esp, 4
    pop ebp
    ret

; void polyos_process_get_args(struct process_arguments *args)
polyos_process_get_args:
    push ebp
    mov ebp, esp
    mov eax, 8 ; Command process_get_args
    push dword [ebp+8] ; args
    int 0x80
    add esp, 4
    pop ebp
    ret

; void polyos_exit()
polyos_exit:
    push ebp
    mov ebp, esp
    mov eax, 9 ; Command exit
    int 0x80
    pop ebp
    ret