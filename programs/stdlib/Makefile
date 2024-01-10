FILES = ./build/start.o ./build/polyos.o ./build/stdlib.o
INCLUDES =-I./src
FLAGS = -g

all: ./build $(FILES) 
	i686-elf-ld -m elf_i386 -relocatable $(FILES) -o ./stdlib.elf

./build:
	mkdir -p ./build

./build/start.o: ./src/start.asm
	nasm -f elf ./src/start.asm -o ./build/start.o

./build/polyos.o: ./src/polyos.asm
	nasm -f elf ./src/polyos.asm -o ./build/polyos.o

./build/stdlib.o: ./src/stdlib.c
	i686-elf-gcc $(FLAGS) $(INCLUDES) -std=gnu99 -c ./src/stdlib.c -o ./build/stdlib.o

clean:
	rm -rf ./build/
	rm -rf ./stdlib.elf