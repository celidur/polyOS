[BITS 32]

section .asm

global serial:function
global print:function
global polyos_getkey:function
global polyos_putchar:function
global polyos_malloc:function
global polyos_free:function
global polyos_process_load_start:function
global polyos_system:function
global polyos_process_get_args:function
global polyos_exit:function
global print_memory:function
global remove_last_char:function
global clear_screen:function

global fopen:function
global fread:function
global fwrite:function
global fseek:function
global fstat:function
global fclose:function

; void serial(char *str)
serial:
    push ebp
    mov ebp, esp
    push dword [ebp+8]
    mov eax, 0 ; Command serial
    int 0x80
    add esp, 4
    pop ebp
    ret

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

; int polyos_process_load_start(const char *filename)
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

; void print_memory()
print_memory:
    push ebp
    mov ebp, esp
    mov eax, 10 ; Command print_memory
    int 0x80
    pop ebp
    ret

; void remove_last_char()
remove_last_char:
    push ebp
    mov ebp, esp
    mov eax, 11 ; Command remove_last_char
    int 0x80
    pop ebp
    ret

; void clear_screen()
clear_screen:
    push ebp
    mov ebp, esp
    mov eax, 12 ; Command clear_screen
    int 0x80
    pop ebp
    ret

; int fopen(const char *filename, const char *mode)
fopen:
    push ebp
    mov ebp, esp
    mov eax, 13 ; Command fopen
    push dword [ebp+12] ; filename
    push dword [ebp+8] ; mode
    int 0x80
    add esp, 8
    pop ebp
    ret

; int fread(int fd, void *buf, size_t size)
fread:
    push ebp
    mov ebp, esp
    mov eax, 14 ; Command fread
    push dword [ebp+16] ; fd
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; size
    int 0x80
    add esp, 12
    pop ebp
    ret

; int fwrite(int fd, void *buf, size_t size)
fwrite:
    push ebp
    mov ebp, esp
    mov eax, 15 ; Command fwrite
    push dword [ebp+16] ; fd
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; size
    int 0x80
    add esp, 12
    pop ebp
    ret

; int fseek(int fd, int offset, FILE_SEEK_MODE mode)
fseek:
    push ebp
    mov ebp, esp
    mov eax, 16 ; Command fseek
    push dword [ebp+16] ; fd
    push dword [ebp+12] ; offset
    push dword [ebp+8] ; mode
    int 0x80
    add esp, 12
    pop ebp
    ret

; int fstat(int fd, struct stat *stat)
fstat:
    push ebp
    mov ebp, esp
    mov eax, 17 ; Command fstat
    push dword [ebp+12] ; fd
    push dword [ebp+8] ; stat
    int 0x80
    add esp, 8
    pop ebp
    ret

; int fclose(int fd)
fclose:
    push ebp
    mov ebp, esp
    mov eax, 18 ; Command fclose
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 4
    pop ebp
    ret
