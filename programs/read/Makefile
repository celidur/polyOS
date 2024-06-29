SRC_DIR = ./src
BUILD_DIR = ./build
STDLIB_PATH = ../stdlib
INCLUDES = -I$(STDLIB_PATH)/include -I./include
FLAGS= -g -ffreestanding -falign-jumps -falign-functions -falign-labels -falign-loops -fstrength-reduce -fomit-frame-pointer -finline-functions -Wno-unused-function -fno-builtin -Werror -Wno-unused-label -Wno-cpp -Wno-unused-parameter -nostdlib -nostartfiles -nodefaultlibs -Wall -O3 -Iinc
BUILDER = i686-elf-gcc
LINKER = i686-elf-ld
NASM = nasm

TARGET = ./read.elf
LINKER_SCRIPT = ./linker.ld

C_SOURCES = $(wildcard $(SRC_DIR)/*.c)
ASM_SOURCES = $(wildcard $(SRC_DIR)/*.asm)
C_OBJS = $(patsubst $(SRC_DIR)/%.c, $(BUILD_DIR)/%.o, $(C_SOURCES))
ASM_OBJS = $(patsubst $(SRC_DIR)/%.asm, $(BUILD_DIR)/%.asm.o, $(ASM_SOURCES))
OBJS = $(C_OBJS) $(ASM_OBJS)

all: $(BUILD_DIR)  $(TARGET)

$(BUILD_DIR):
	@mkdir -p $(BUILD_DIR)

$(TARGET): $(OBJS)
	$(BUILDER) -g -T $(LINKER_SCRIPT) -o $(TARGET) -ffreestanding -O0 -nostdlib -fpic -g $(OBJS) $(STDLIB_PATH)/stdlib.elf

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
	$(BUILDER) $(FLAGS) -c $< -o $@ $(INCLUDES)

$(BUILD_DIR)/%.asm.o: $(SRC_DIR)/%.asm
	$(NASM) -f elf $< -o $@

clean:
	if [ -d "$(BUILD_DIR)" ]; then rm -rf $(BUILD_DIR); fi
	if [ -f "$(TARGET)" ]; then rm -rf $(TARGET); fi