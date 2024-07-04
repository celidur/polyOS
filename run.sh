# /bin/bash

# compile if any argument is passed
if [ "$1" ] || [ ! -f "./bin/os.bin" ]; then
    make clean
    sh ./build.sh
fi

qemu-system-x86_64 -drive format=raw,file=./bin/os.bin -serial stdio 2>&1 | tee "log/log_$(date +'%Y%m%d_%H%M%S').txt"