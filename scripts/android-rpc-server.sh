#!/bin/sh
# Build deltachat-rpc-server for Android.

set -e

test -n "$ANDROID_NDK_ROOT" || exit 1

RUSTUP_TOOLCHAIN="1.64.0"
rustup install "$RUSTUP_TOOLCHAIN"
rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android --toolchain "$RUSTUP_TOOLCHAIN"

KERNEL="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
NDK_HOST_TAG="$KERNEL-$ARCH"
TOOLCHAIN="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$NDK_HOST_TAG"

PACKAGE="deltachat-rpc-server"

export CARGO_PROFILE_RELEASE_LTO=on

CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$TOOLCHAIN/bin/armv7a-linux-androideabi16-clang" \
	TARGET_CC="$TOOLCHAIN/bin/armv7a-linux-androideabi16-clang" \
	TARGET_AR="$TOOLCHAIN/bin/llvm-ar" \
	TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib" \
	cargo "+$RUSTUP_TOOLCHAIN" rustc --release --target armv7-linux-androideabi -p $PACKAGE

CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/aarch64-linux-android21-clang" \
	TARGET_CC="$TOOLCHAIN/bin/aarch64-linux-android21-clang" \
	TARGET_AR="$TOOLCHAIN/bin/llvm-ar" \
	TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib" \
	cargo "+$RUSTUP_TOOLCHAIN" rustc --release --target aarch64-linux-android -p $PACKAGE

CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/i686-linux-android16-clang" \
	TARGET_CC="$TOOLCHAIN/bin/i686-linux-android16-clang" \
	TARGET_AR="$TOOLCHAIN/bin/llvm-ar" \
	TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib" \
	cargo "+$RUSTUP_TOOLCHAIN" rustc --release --target i686-linux-android -p $PACKAGE

CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/x86_64-linux-android21-clang" \
	TARGET_CC="$TOOLCHAIN/bin/x86_64-linux-android21-clang" \
	TARGET_AR="$TOOLCHAIN/bin/llvm-ar" \
	TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib" \
	cargo "+$RUSTUP_TOOLCHAIN" rustc --release --target x86_64-linux-android -p $PACKAGE
