# The `raylib` library's runtime for C++ (libs/raylib/real/cpp), included by the app's CMakeLists.txt: the
# adapters of `foreign c "raylib#Name"` and the screen of the `game` library's window loop.
#
# raylib 5.5 is linked when the app is built and found again at run time through a run path relative to the
# executable, so the checkout can be moved. RAYLIB_ROOT holds include/raylib.h and lib/libraylib.so.
set(RAYLIB_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../vendor/raylib" CACHE PATH "raylib 5.5: include/ and lib/")
if(NOT EXISTS "${RAYLIB_ROOT}/lib/libraylib.so" OR NOT EXISTS "${RAYLIB_ROOT}/include/raylib.h")
  message(FATAL_ERROR "raylib not found in ${RAYLIB_ROOT}: build it there, or set RAYLIB_ROOT")
endif()
add_library(raylib SHARED IMPORTED)
set_target_properties(raylib PROPERTIES IMPORTED_LOCATION "${RAYLIB_ROOT}/lib/libraylib.so")
# An imported target's headers are system headers: raylib.h is C and is not held to -Wpedantic.
target_include_directories(raylib INTERFACE "${RAYLIB_ROOT}/include")
file(RELATIVE_PATH raylib_from_build "${CMAKE_BINARY_DIR}" "${RAYLIB_ROOT}/lib")
set(CMAKE_SKIP_BUILD_RPATH ON)
target_link_options(raylib INTERFACE "LINKER:-rpath,\$ORIGIN/${raylib_from_build}")

add_library(lib_raylib STATIC
  lib/raylib/binding.cpp
  lib/raylib/convert.cpp
)
target_include_directories(lib_raylib PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(lib_raylib PUBLIC program lib_game PRIVATE raylib)
target_link_libraries(app PRIVATE lib_raylib)

# The runtime's own test, through the real library; it needs no window.
add_executable(lib_raylib_test lib/raylib/tests/edge_test.cpp)
target_link_libraries(lib_raylib_test PRIVATE lib_raylib program_real)
add_test(NAME lib_raylib COMMAND lib_raylib_test)
