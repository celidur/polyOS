PARENT_DIR := $(shell basename $(shell pwd))
TARGET = ./$(PARENT_DIR).elf

all:  $(TARGET)

$(TARGET): FORCE
	cargo build -Zjson-target-spec --release
	cp ./target/i686-polyos/release/$(PARENT_DIR) $(TARGET)

clean:
# 	cargo clean
	rm -f $(TARGET)

FORCE:

.PHONY: FORCE
