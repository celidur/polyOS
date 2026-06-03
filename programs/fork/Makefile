PARENT_DIR := $(shell basename $(shell pwd))
TARGET = ./$(PARENT_DIR).elf

all:  $(TARGET)

$(TARGET): FORCE
	cd ../
	cargo build -Zjson-target-spec --release
	cp ./target/i686-polyos/release/$(PARENT_DIR) $(TARGET)

FORCE:

clean:
# 	cargo clean
	rm -f $(TARGET)
