FILES = ./build/blank.o
INCLUDES = -I../stdlib/src
FLAGS= -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O0 -Iinc
BUILDER = i686-elf-gcc
LINKER = i686-elf-ld

all: ./build $(FILES)
	$(BUILDER) -g -T ./linker.ld -o ./blank.elf -ffreestanding -O0 -nostdlib -fpic -g $(FILES) ../stdlib/stdlib.elf

./build:
	mkdir -p ./build

./build/blank.o: ./blank.c
	$(BUILDER) $(INCLUDES) -I./ $(FLAGS) -std=gnu99 -c ./blank.c -o ./build/blank.o

clean:
	rm -rf ./build/
	rm -rf ./blank.elf