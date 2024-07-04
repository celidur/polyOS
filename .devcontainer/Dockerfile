# Stage 1: Build
FROM alpine AS build

RUN apk update && \
    apk add --no-cache \
    wget \
    build-base \
    bison \
    flex \
    gmp-dev \
    mpfr-dev \
    mpc1-dev \
    texinfo \
    isl-dev \
    make \
    nasm

# Preparation
ENV PREFIX /opt/cross
ENV TARGET i686-elf
ENV PATH $PREFIX/bin:$PATH

RUN mkdir -p /src && \
    mkdir -p $PREFIX

# Binutils
RUN cd /src && \
    wget https://ftp.gnu.org/gnu/binutils/binutils-2.35.tar.gz && \
    tar -xvf binutils-2.35.tar.gz && \
    cd binutils-2.35 && \
    mkdir build-binutils && \
    cd build-binutils && \
    ../configure --target=$TARGET --prefix="$PREFIX" --with-sysroot --disable-nls --disable-werror && \
    make -j$(nproc) && \
    make install

# GCC
RUN cd /src && \
    wget https://ftp.gnu.org/gnu/gcc/gcc-10.2.0/gcc-10.2.0.tar.gz && \
    tar -xvf gcc-10.2.0.tar.gz && \
    cd gcc-10.2.0 && \
    mkdir build-gcc && \
    cd build-gcc && \
    ../configure --target=$TARGET --prefix="$PREFIX" --disable-nls --enable-languages=c,c++ --without-headers && \
    make all-gcc -j$(nproc) && \
    make all-target-libgcc -j$(nproc) && \
    make install-gcc && \
    make install-target-libgcc

# Stage 2: Create final image with only the resulting toolchain
FROM alpine

ENV PREFIX /opt/cross
ENV TARGET i686-elf
ENV PATH $PREFIX/bin:$PATH

# Copy the built toolchain from the build stage
COPY --from=build $PREFIX $PREFIX

# Install additional tools
RUN apk add --no-cache \
    libgcc \
    libstdc++ \
    gmp \
    mpfr \
    mpc1 \
    isl-dev \
    sudo \
    make \
    nasm \
    git \
    qemu-system-x86_64 \
    openbox \
    xfce4-terminal \
    xauth \
    xorg-server \
    xvfb \
    x11vnc \
    websockify

# Install noVNC
RUN mkdir -p /usr/share/novnc && \
    wget -O /tmp/novnc.tar.gz https://github.com/novnc/noVNC/archive/refs/tags/v1.3.0.tar.gz && \
    tar -xzf /tmp/novnc.tar.gz -C /usr/share/novnc --strip-components=1 && \
    rm /tmp/novnc.tar.gz

# Configure x11vnc and noVNC
COPY setup-vnc.sh /setup-vnc.sh

RUN chmod +x /setup-vnc.sh 

# # Copy and set up OpenRC service
# COPY my-xvfb /etc/init.d/my-xvfb
# RUN chmod +x /etc/init.d/my-xvfb && \
#     rc-update add my-xvfb default

# Verify installation
RUN $TARGET-gcc --version && \
    $TARGET-ld --version

# Clean up unnecessary files to keep the image small
RUN rm -rf /var/cache/apk/* /tmp/* /var/tmp/*

# Expose necessary ports
EXPOSE 5900 8080

CMD ["/bin/sh", "-c", "/setup-vnc.sh & tail -f /dev/null"]
