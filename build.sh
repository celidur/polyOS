# /bin/bash
os_type=$(uname -s)
# for mac brew install i686-elf-gcc
# for linux compile from source
if [ $os_type == "Linux" ]; then
    export PREFIX="$HOME/opt/cross"
    export TARGET=i686-elf
    export PATH="$PREFIX/bin:$PATH"
fi
make all