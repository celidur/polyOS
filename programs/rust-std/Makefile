CC = i686-elf-gcc
AR = i686-elf-ar
OBJCOPY = i686-elf-objcopy
FLAGS = -O2 -ffreestanding -fno-builtin -fno-stack-protector -fno-pie -I../stdlib/include -I../stdlib/src
LIBRARIES = c util rt pthread m dl gcc_eh
RENAMES = _start=__polyos_legacy_start \
    c_start=__polyos_legacy_c_start \
    sigaction=__polyos_sigaction \
    sigprocmask=__polyos_sigprocmask \
    pthread_sigmask=__polyos_pthread_sigmask \
    sigemptyset=__polyos_sigemptyset \
    sigfillset=__polyos_sigfillset \
    sigaddset=__polyos_sigaddset \
    sigdelset=__polyos_sigdelset \
    sigismember=__polyos_sigismember \
    opendir=__polyos_opendir \
    closedir=__polyos_closedir \
    readdir=__polyos_readdir \
    tcgetattr=__polyos_tcgetattr \
    tcsetattr=__polyos_tcsetattr \
    pthread_attr_init=__polyos_pthread_attr_init \
    pthread_create=__polyos_pthread_create

all: lib/runtime.o lib/start.o $(addprefix lib/lib,$(addsuffix .a,$(LIBRARIES)))

lib/runtime.o: ../stdlib/stdlib.elf Makefile
	@mkdir -p lib
	$(OBJCOPY) $(foreach symbol,$(RENAMES),--redefine-sym $(symbol)) $< $@

lib/compat.o: compat.c Makefile
	@mkdir -p lib
	$(CC) $(FLAGS) -c $< -o $@

lib/start.o: start.asm
	nasm -f elf $< -o $@

lib/syscall.o: syscall.asm
	nasm -f elf $< -o $@

lib/libc.a: lib/compat.o lib/syscall.o
	$(AR) rcs $@ $^

lib/lib%.a:
	@mkdir -p lib
	$(AR) rcs $@

clean:
	rm -f lib/*.o lib/*.a
