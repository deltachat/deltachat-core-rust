cmake_minimum_required(VERSION 3.16)
project(deltachat)

find_program(CARGO cargo)

add_custom_command(
	OUTPUT
	"target/release/libdeltachat.a"
	"target/release/libdeltachat.so"
	"target/release/pkgconfig/deltachat.pc"
	COMMAND PREFIX=${CMAKE_INSTALL_PREFIX} ${CARGO} build --package deltachat_ffi --release
	WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

add_custom_target(
	lib_deltachat
	ALL
	DEPENDS
	"target/release/libdeltachat.a"
	"target/release/libdeltachat.so"
	"target/release/pkgconfig/deltachat.pc"
)

include(GNUInstallDirs)
install(FILES "deltachat-ffi/deltachat.h" DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES "target/release/libdeltachat.a" DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES "target/release/libdeltachat.so" DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES "target/release/pkgconfig/deltachat.pc" DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)
