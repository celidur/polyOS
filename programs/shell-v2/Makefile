PARENT_DIR := $(shell basename $(shell pwd))
TARGET = ./$(PARENT_DIR).elf

all:  $(TARGET)

$(TARGET): 
	cd ../
	cargo build --release
	cp ./target/i686-polyos/release/$(PARENT_DIR) $(TARGET)

clean:
	if [ -f "$(TARGET)" ]; then rm -rf $(TARGET); fi