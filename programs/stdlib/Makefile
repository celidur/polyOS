FILES = ./build/start.o ./build/start.asm.o ./build/polyos.asm.o ./build/polyos.o ./build/stdlib.o ./build/stdio.o ./build/memory.o ./build/string.o
INCLUDES =-I./src
FLAGS = -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O3 -Iinc
BUILDER = i686-elf-gcc
LINKER = i686-elf-ld

all: ./build $(FILES) 
	$(LINKER) -m elf_i386 -relocatable $(FILES) -o ./stdlib.elf

./build:
	mkdir -p ./build

./build/start.asm.o: ./src/start.asm
	nasm -f elf ./src/start.asm -o ./build/start.asm.o

./build/start.o: ./src/start.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/start.c -o ./build/start.o

./build/polyos.asm.o: ./src/polyos.asm
	nasm -f elf ./src/polyos.asm -o ./build/polyos.asm.o

./build/polyos.o: ./src/polyos.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/polyos.c -o ./build/polyos.o

./build/stdlib.o: ./src/stdlib.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/stdlib.c -o ./build/stdlib.o

./build/stdio.o: ./src/stdio.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/stdio.c -o ./build/stdio.o

./build/memory.o: ./src/memory.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/memory.c -o ./build/memory.o

./build/string.o: ./src/string.c
	$(BUILDER) $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/string.c -o ./build/string.o

clean:
	if [ -d "./build" ]; then rm -rf ./build; fi
	if [ -f "./stdlib.elf" ]; then rm -rf ./stdlib.elf; fi