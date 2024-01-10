FILES = ./build/shell.o
INCLUDES = -I../stdlib/src
FLAGS= -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O0 -Iinc


all: ./build $(FILES)
	i686-elf-gcc -g -T ./linker.ld -o ./shell.elf -ffreestanding -O0 -nostdlib -fpic -g $(FILES) ../stdlib/stdlib.elf

./build:
	mkdir -p ./build

./build/shell.o: ./src/shell.c
	i686-elf-gcc $(FLAGS) -c ./src/shell.c -o ./build/shell.o $(INCLUDES)

clean:
	rm -rf ./build/
	rm -rf ./shell.elf