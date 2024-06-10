# /bin/bash
# supposing that we using apt 
sudo apt-get update
sudo apt-get install -y nasm build-essential bison flex libgmp3-dev libmpc-dev libmpfr-dev texinfo libisl-dev qemu-system-x86

mkdir -p ~/opt
mkdir -p ~/src
cd ~/src
wget https://ftp.gnu.org/gnu/binutils/binutils-2.35.tar.gz
tar -xvf binutils-2.35.tar.gz
cd binutils-2.35
mkdir build
cd build
../configure --target=$TARGET --prefix="$PREFIX" --with-sysroot --disable-nls --disable-werror
make -j$(nproc)
make install
cd ~/src
wget https://ftp.gnu.org/gnu/gcc/gcc-10.2.0/gcc-10.2.0.tar.gz

if command -v $TARGET-as > /dev/null; then
    echo "binutils installed"
else
    echo "binutils not installed"
    exit 1
fi

tar -xvf gcc-10.2.0.tar.gz
cd gcc-10.2.0
mkdir build
cd build
../configure --target=$TARGET --prefix="$PREFIX" --disable-nls --enable-languages=c,c++ --without-headers
make all-gcc -j$(nproc)
make all-target-libgcc -j$(nproc)
make install-gcc
make install-target-libgcc