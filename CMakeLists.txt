cmake_minimum_required(VERSION 3.16)
project(deltachat LANGUAGES C)
include(GNUInstallDirs)

find_program(CARGO cargo)

add_custom_command(
	OUTPUT
	"target/release/libdeltachat.a"
	"target/release/libdeltachat.so"
	"target/release/pkgconfig/deltachat.pc"
        COMMAND
        PREFIX=${CMAKE_INSTALL_PREFIX}
        LIBDIR=${CMAKE_INSTALL_FULL_LIBDIR}
        INCLUDEDIR=${CMAKE_INSTALL_FULL_INCLUDEDIR}
        ${CARGO} build --release --no-default-features

	# Build in `deltachat-ffi` directory instead of using
	# `--package deltachat_ffi` to avoid feature resolver version
	# "1" bug which makes `--no-default-features` affect only
	# `deltachat`, but not `deltachat-ffi` package.
	#
	# We can't enable version "2" resolver [1] because it is not
	# stable yet on rust 1.50.0.
	#
	# [1] https://doc.rust-lang.org/nightly/cargo/reference/features.html#resolver-version-2-command-line-flags
	WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/deltachat-ffi
)

add_custom_target(
	lib_deltachat
	ALL
	DEPENDS
	"target/release/libdeltachat.a"
	"target/release/libdeltachat.so"
	"target/release/pkgconfig/deltachat.pc"
)

install(FILES "deltachat-ffi/deltachat.h" DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES "target/release/libdeltachat.a" DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES "target/release/libdeltachat.so" DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES "target/release/pkgconfig/deltachat.pc" DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)
