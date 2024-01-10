FILES = ./build/start.o ./build/start.asm.o ./build/polyos.asm.o ./build/polyos.o ./build/stdlib.o ./build/stdio.o ./build/memory.o ./build/string.o
INCLUDES =-I./src
FLAGS = -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O0 -Iinc


all: ./build $(FILES) 
	i686-elf-ld -m elf_i386 -relocatable $(FILES) -o ./stdlib.elf

./build:
	mkdir -p ./build

./build/start.asm.o: ./src/start.asm
	nasm -f elf ./src/start.asm -o ./build/start.asm.o

./build/start.o: ./src/start.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/start.c -o ./build/start.o

./build/polyos.asm.o: ./src/polyos.asm
	nasm -f elf ./src/polyos.asm -o ./build/polyos.asm.o

./build/polyos.o: ./src/polyos.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/polyos.c -o ./build/polyos.o

./build/stdlib.o: ./src/stdlib.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/stdlib.c -o ./build/stdlib.o

./build/stdio.o: ./src/stdio.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/stdio.c -o ./build/stdio.o

./build/memory.o: ./src/memory.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/memory.c -o ./build/memory.o

./build/string.o: ./src/string.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/string.c -o ./build/string.o

clean:
	rm -rf ./build/
	rm -rf ./stdlib.elf