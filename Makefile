FILES = ./build/kernel.asm.o ./build/kernel.o ./build/idt/idt.asm.o ./build/idt/idt.o ./build/memory/memory.o ./build/io/io.o ./build/memory/heap/heap.o ./build/memory/heap/kheap.o ./build/memory/paging/paging.o ./build/memory/paging/paging.asm.o ./build/disk/disk.o ./build/string/string.o ./build/fs/pparser.o ./build/disk/streamer.o ./build/fs/file.o ./build/fs/fat/fat16.o ./build/gdt/gdt.o ./build/gdt/gdt.asm.o ./build/task/tss.asm.o ./build/task/task.o ./build/task/task.asm.o ./build/task/process.o ./build/int80h/int80.o ./build/int80h/misc.o ./build/int80h/io.o ./build/keyboard/keyboard.o ./build/keyboard/classic.o ./build/loader/formats/elf.o ./build/loader/formats/elfloader.o ./build/int80h/heap.o ./build/int80h/process.o ./build/terminal/terminal.o
INCLUDES = -I./src
FLAGS = -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O0 -Iinc
BUILDER = i686-elf-gcc
LINKER = i686-elf-ld
DIRECTORIES = ./bin $(foreach dir, $(dir $(FILES)), $(dir $(dir)))
# remove redundant directories
DIRECTORIES := $(sort $(DIRECTORIES))
OS := $(shell uname -s)

all: $(DIRECTORIES) ./bin/boot.bin ./bin/kernel.bin 
	rm -rf ./bin/os.bin
	dd if=./bin/boot.bin >> ./bin/os.bin
	dd if=./bin/kernel.bin >> ./bin/os.bin
	dd if=/dev/zero bs=1048576 count=16 >> ./bin/os.bin

	# run mount-disk
	rm -rf ./mnt
	mkdir -p ./mnt/d
ifeq ($(OS), Darwin)
	@echo "Mounting disk image..."
	$(eval DISK := $(shell hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount ./bin/os.bin))
	sudo mount -t msdos $(DISK) ./mnt/d
else
	sudo mount -t vfat ./bin/os.bin ./mnt/d
endif

	# Copy FILES
	sudo cp ./hello.txt ./mnt/d
	sudo cp ./programs/blank/blank.elf ./mnt/d
	sudo cp ./programs/shell/shell.elf ./mnt/d

	sudo umount ./mnt/d
ifeq ($(OS), Darwin)
	hdiutil detach  $(DISK)
endif
	rm -rf ./mnt



$(DIRECTORIES):
	mkdir -p $(DIRECTORIES)

./bin/kernel.bin: ./bin/ $(FILES) user_programs
	$(LINKER) -g -relocatable $(FILES) -o ./build/kernelfull.o
	$(BUILDER) $(FLAGS) -T ./src/linker.ld -o ./bin/kernel.bin -ffreestanding -O0 -nostdlib ./build/kernelfull.o

./bin/boot.bin: ./src/boot/boot.asm
	nasm -f bin ./src/boot/boot.asm -o ./bin/boot.bin

./build/kernel.asm.o: ./src/kernel.asm
	nasm -f elf -g ./src/kernel.asm -o ./build/kernel.asm.o

./build/kernel.o: ./src/kernel.c
	$(BUILDER) $(INCLUDES) $(FLAGS) -std=gnu99 -c ./src/kernel.c -o ./build/kernel.o

./build/idt/idt.asm.o: ./src/idt/idt.asm
	nasm -f elf -g ./src/idt/idt.asm -o ./build/idt/idt.asm.o

./build/idt/idt.o: ./src/idt/idt.c
	$(BUILDER) $(INCLUDES) -I ./src/idt $(FLAGS) -std=gnu99 -c ./src/idt/idt.c -o ./build/idt/idt.o

./build/memory/memory.o: ./src/memory/memory.c
	$(BUILDER) $(INCLUDES) -I ./src/memory $(FLAGS) -std=gnu99 -c ./src/memory/memory.c -o ./build/memory/memory.o

./build/io/io.o: ./src/io/io.asm
	nasm -f elf -g ./src/io/io.asm -o ./build/io/io.o

./build/memory/heap/heap.o: ./src/memory/heap/heap.c
	$(BUILDER) $(INCLUDES) -I ./src/memory/heap $(FLAGS) -std=gnu99 -c ./src/memory/heap/heap.c -o ./build/memory/heap/heap.o

./build/memory/heap/kheap.o: ./src/memory/heap/kheap.c
	$(BUILDER) $(INCLUDES) -I ./src/memory/heap $(FLAGS) -std=gnu99 -c ./src/memory/heap/kheap.c -o ./build/memory/heap/kheap.o

./build/memory/paging/paging.o: ./src/memory/paging/paging.c
	$(BUILDER) $(INCLUDES) -I ./src/memory/paging $(FLAGS) -std=gnu99 -c ./src/memory/paging/paging.c -o ./build/memory/paging/paging.o

./build/memory/paging/paging.asm.o: ./src/memory/paging/paging.asm
	nasm -f elf -g ./src/memory/paging/paging.asm -o ./build/memory/paging/paging.asm.o

./build/disk/disk.o: ./src/disk/disk.c
	$(BUILDER) $(INCLUDES) -I ./src/disk $(FLAGS) -std=gnu99 -c ./src/disk/disk.c -o ./build/disk/disk.o

./build/string/string.o: ./src/string/string.c
	$(BUILDER) $(INCLUDES) -I ./src/string $(FLAGS) -std=gnu99 -c ./src/string/string.c -o ./build/string/string.o

./build/fs/pparser.o: ./src/fs/pparser.c
	$(BUILDER) $(INCLUDES) -I ./src/fs $(FLAGS) -std=gnu99 -c ./src/fs/pparser.c -o ./build/fs/pparser.o

./build/disk/streamer.o: ./src/disk/streamer.c
	$(BUILDER) $(INCLUDES) -I ./src/disk $(FLAGS) -std=gnu99 -c ./src/disk/streamer.c -o ./build/disk/streamer.o

./build/fs/file.o: ./src/fs/file.c
	$(BUILDER) $(INCLUDES) -I ./src/fs $(FLAGS) -std=gnu99 -c ./src/fs/file.c -o ./build/fs/file.o

./build/fs/fat/fat16.o: ./src/fs/fat/fat16.c
	$(BUILDER) $(INCLUDES) -I ./src/fs/fat $(FLAGS) -std=gnu99 -c ./src/fs/fat/fat16.c -o ./build/fs/fat/fat16.o

./build/gdt/gdt.o: ./src/gdt/gdt.c
	$(BUILDER) $(INCLUDES) -I ./src/gdt $(FLAGS) -std=gnu99 -c ./src/gdt/gdt.c -o ./build/gdt/gdt.o

./build/gdt/gdt.asm.o: ./src/gdt/gdt.asm
	nasm -f elf -g ./src/gdt/gdt.asm -o ./build/gdt/gdt.asm.o

./build/task/tss.asm.o: ./src/task/tss.asm
	nasm -f elf -g ./src/task/tss.asm -o ./build/task/tss.asm.o

./build/task/task.o: ./src/task/task.c
	$(BUILDER) $(INCLUDES) -I ./src/task $(FLAGS) -std=gnu99 -c ./src/task/task.c -o ./build/task/task.o

./build/task/task.asm.o: ./src/task/task.asm
	nasm -f elf -g ./src/task/task.asm -o ./build/task/task.asm.o

./build/task/process.o: ./src/task/process.c
	$(BUILDER) $(INCLUDES) -I ./src/task $(FLAGS) -std=gnu99 -c ./src/task/process.c -o ./build/task/process.o

./build/int80h/int80.o: ./src/int80h/int80.c
	$(BUILDER) $(INCLUDES) -I ./src/int80h $(FLAGS) -std=gnu99 -c ./src/int80h/int80.c -o ./build/int80h/int80.o

./build/int80h/misc.o: ./src/int80h/misc.c
	$(BUILDER) $(INCLUDES) -I ./src/int80h $(FLAGS) -std=gnu99 -c ./src/int80h/misc.c -o ./build/int80h/misc.o

./build/int80h/io.o: ./src/int80h/io.c
	$(BUILDER) $(INCLUDES) -I ./src/int80h $(FLAGS) -std=gnu99 -c ./src/int80h/io.c -o ./build/int80h/io.o

./build/keyboard/keyboard.o: ./src/keyboard/keyboard.c
	$(BUILDER) $(INCLUDES) -I ./src/keyboard $(FLAGS) -std=gnu99 -c ./src/keyboard/keyboard.c -o ./build/keyboard/keyboard.o

./build/keyboard/classic.o: ./src/keyboard/classic.c
	$(BUILDER) $(INCLUDES) -I ./src/keyboard $(FLAGS) -std=gnu99 -c ./src/keyboard/classic.c -o ./build/keyboard/classic.o

./build/loader/formats/elf.o: ./src/loader/formats/elf.c
	$(BUILDER) $(INCLUDES) -I ./src/loader/formats $(FLAGS) -std=gnu99 -c ./src/loader/formats/elf.c -o ./build/loader/formats/elf.o

./build/loader/formats/elfloader.o: ./src/loader/formats/elfloader.c
	$(BUILDER) $(INCLUDES) -I ./src/loader/formats $(FLAGS) -std=gnu99 -c ./src/loader/formats/elfloader.c -o ./build/loader/formats/elfloader.o

./build/int80h/heap.o: ./src/int80h/heap.c
	$(BUILDER) $(INCLUDES) -I ./src/int80h $(FLAGS) -std=gnu99 -c ./src/int80h/heap.c -o ./build/int80h/heap.o

./build/int80h/process.o: ./src/int80h/process.c
	$(BUILDER) $(INCLUDES) -I ./src/int80h $(FLAGS) -std=gnu99 -c ./src/int80h/process.c -o ./build/int80h/process.o

./build/terminal/terminal.o: ./src/terminal/terminal.c
	$(BUILDER) $(INCLUDES) -I ./src/terminal $(FLAGS) -std=gnu99 -c ./src/terminal/terminal.c -o ./build/terminal/terminal.o

clean: user_programs_clean
	if [ -d "./bin" ]; then rm -rf ./bin; fi
	if [ -d "./build" ]; then rm -rf ./build; fi

user_programs:
	cd ./programs/stdlib && make all
	cd ./programs/blank && make all
	cd ./programs/shell && make all

user_programs_clean:
	cd ./programs/stdlib && make clean
	cd ./programs/blank && make clean
	cd ./programs/shell && make clean
