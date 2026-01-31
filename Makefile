BIN_DIR = ./bin
BUILD_DIR = ./build
SRC_DIR = ./src

RUST_DIR = ./src/rust
RUST_TARGET = i686-polyos
RUST_KERNEL = $(RUST_DIR)/target/$(RUST_TARGET)/release/rust_kernel

NASM = nasm

DIRECTORIES = $(BIN_DIR) $(sort $(dir $(OBJ_FILES)))

OS := $(shell uname -s)

PROGRAM_DIRS := $(wildcard ./programs/*/)
PROGRAM_NAMES := $(filter-out stdlib, $(notdir $(patsubst %/,%,$(PROGRAM_DIRS))))

all: $(DIRECTORIES) $(BIN_DIR)/os.bin user_programs
	mkdir -p ./log
	@echo "Mounting disk image..."

	# run mount-disk
	if [ -d "./mnt" ]; then rm -rf ./mnt; fi
	mkdir -p ./mnt/d
ifeq ($(OS), Darwin)
	DEV_ID=$$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount $(BIN_DIR)/os.bin | grep "/dev/disk" | sed -E 's/ .*//') && \
	diskutil mount -mountPoint ./mnt/d $$DEV_ID && \
	cp -r ./file/* ./mnt/d; \
	sleep 1; \
	diskutil unmountDisk $$DEV_ID; \
	hdiutil detach $$DEV_ID;
else
	sudo mount -t vfat $(BIN_DIR)/os.bin ./mnt/d

	# Copy file
	sudo cp -r ./file/* ./mnt/d

	sudo umount ./mnt/d
endif
	rm -rf ./mnt

$(BIN_DIR)/:
	@mkdir -p $@

$(DIRECTORIES):
	@mkdir -p $(DIRECTORIES)

$(RUST_KERNEL): $(RUST_DIR)/Cargo.toml
	cd $(RUST_DIR) && cargo +nightly build --release --target $(RUST_TARGET).json

$(BIN_DIR)/os.bin: $(BIN_DIR)/boot.bin $(BIN_DIR)/kernel.bin
	@rm -f $@
	@dd if=$(BIN_DIR)/boot.bin >> $@
	@dd if=$(BIN_DIR)/kernel.bin >> $@
	@echo "Modifying ReservedSectors in boot.bin..."
	@size_in_bytes=$(shell wc -c < $(BIN_DIR)/kernel.bin | awk '{print $$1}') && \
	size_in_sectors=$$((($$size_in_bytes + 1023) / 512)) && \
	printf "Calculated size_in_sectors: 0x%04X\n" $$size_in_sectors && \
	low_byte=$$(printf '%04X' $$size_in_sectors | cut -c3-4) && \
	high_byte=$$(printf '%04X' $$size_in_sectors | cut -c1-2) && \
	printf "\x$$low_byte" | dd of=$@ bs=1 seek=14 count=1 conv=notrunc && \
	printf "\x$$high_byte" | dd of=$@ bs=1 seek=15 count=1 conv=notrunc
	@dd if=/dev/zero bs=1048576 count=16 >> $@
	@echo "OS image created."

$(BIN_DIR)/kernel.bin: $(BIN_DIR)/ $(OBJ_FILES) $(RUST_KERNEL) user_programs
	cp $(RUST_KERNEL) $(BIN_DIR)/kernel.bin

$(BIN_DIR)/boot.bin: $(SRC_DIR)/boot/boot.asm
	$(NASM) -f bin $< -o $@

clean: user_programs_clean
	rm -rf $(BIN_DIR) $(BUILD_DIR)
	rm -rf ./file/bin/*.elf
	rm -rf $(RUST_KERNEL)
# 	cd $(RUST_DIR) && cargo clean

user_programs: ./file/bin $(PROGRAM_NAMES)

./file/bin:
	@mkdir -p ./file/bin

stdlib:
	+$(MAKE) -C programs/stdlib all

$(PROGRAM_NAMES): stdlib
	@if [ -f programs/$@/Makefile ]; then \
		$(MAKE) -C programs/$@ all; \
		cp programs/$@/$@.elf ./file/bin/$@.elf; \
	fi


user_programs_clean:
	$(foreach dir,$(PROGRAM_DIRS), \
		if [ -f $(dir)/Makefile ]; then \
			$(MAKE) -C $(dir) clean; \
		fi;)


clean_log:
	if [ -d "./log" ]; then rm ./log/*; fi

.PHONY: all clean clean_log