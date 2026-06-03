# /bin/bash

NEEDS_BUILD=0

# compile if any argument is passed
if [ "$1" ] || [ ! -f "./bin/os.bin" ]; then
    NEEDS_BUILD=1
elif find ./src ./programs ./file ./Makefile ./build.sh -newer ./bin/os.bin -print -quit | grep -q .; then
    NEEDS_BUILD=1
fi

if [ "$NEEDS_BUILD" -eq 1 ]; then
    make clean
    sh ./build.sh
fi

mkdir -p log

qemu-system-x86_64 \
    -drive format=raw,file=./bin/os.bin \
    -netdev user,id=net0 \
    -device rtl8139,netdev=net0 \
    -serial stdio \
    2>&1 | tee "log/log_$(date +'%Y%m%d_%H%M%S').txt"
