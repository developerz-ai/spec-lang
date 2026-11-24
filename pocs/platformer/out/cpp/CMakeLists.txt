# Projected by hand from app SkyhopCpp, @target(cpp), @bin("skyhop") (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
cmake_minimum_required(VERSION 3.28)
project(skyhop LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

add_compile_options(-Wall -Wextra -Wpedantic -Werror)

# Release: whole-program optimisation (@why "the game as one native binary").
include(CheckIPOSupported)
check_ipo_supported(RESULT ipo_ok OUTPUT ipo_message LANGUAGES CXX)
if(ipo_ok)
  set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)
endif()

# raylib 5.5, the one copy every backend uses (docs/contract.md section 3): linked at build time,
# found again at run time through a run path relative to the executable. SKYHOP_RAYLIB is ignored.
set(SKYHOP_RAYLIB_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../vendor/raylib" CACHE PATH "raylib built by bin/raylib")
if(NOT EXISTS "${SKYHOP_RAYLIB_ROOT}/lib/libraylib.so" OR NOT EXISTS "${SKYHOP_RAYLIB_ROOT}/include/raylib.h")
  message(FATAL_ERROR "raylib not found in ${SKYHOP_RAYLIB_ROOT}: run pocs/platformer/bin/raylib")
endif()
add_library(raylib SHARED IMPORTED)
set_target_properties(raylib PROPERTIES IMPORTED_LOCATION "${SKYHOP_RAYLIB_ROOT}/lib/libraylib.so")
# An imported target's headers are system headers: raylib.h is C and is not held to -Wpedantic.
target_include_directories(raylib INTERFACE "${SKYHOP_RAYLIB_ROOT}/include")
# The run path is relative to the executable (build/), so the checkout can be moved.
set(CMAKE_SKIP_BUILD_RPATH ON)
target_link_options(raylib INTERFACE "LINKER:-rpath,\$ORIGIN/../../../vendor/raylib/lib")

# hop.geom, hop.model, hop.levels, hop.physics, hop.world, hop.render: pure. This target links
# nothing, so the simulation cannot reach raylib, the clock or a stream.
add_library(hop_sim STATIC
  src/geom/text.cpp
  src/world/lines.cpp
  src/world/start.cpp
  src/world/lose.cpp
  src/world/step.cpp
  src/world/summary.cpp
  src/render/scene.cpp
  src/render/text.cpp
)
target_include_directories(hop_sim PUBLIC include)

# hop.screen: the only code that includes raylib.h.
add_library(hop_screen STATIC
  src/screen/edge.cpp
  src/screen/assets.cpp
  src/screen/present.cpp
  src/screen/read_pad.cpp
  src/screen/session.cpp
)
target_link_libraries(hop_screen PUBLIC hop_sim PRIVATE raylib)

# The game vocabulary: arguments, the input script, the four commands.
add_library(game_cli STATIC src/cli/parser.cpp)
target_link_libraries(game_cli PUBLIC hop_sim)

add_library(game_script STATIC src/script/reader.cpp)
target_link_libraries(game_script PUBLIC hop_sim)

add_library(game_app STATIC
  src/app/fnv.cpp
  src/app/io.cpp
  src/app/replay.cpp
  src/app/run.cpp
  src/app/draw.cpp
  src/app/shot.cpp
  src/app/play.cpp
  src/app/program.cpp
)
target_link_libraries(game_app PUBLIC game_cli game_script hop_screen)

add_executable(skyhop src/app/main.cpp)
target_link_libraries(skyhop PRIVATE game_app)

# Tests: one executable per spec concern, a small harness, no framework, no mocks.
enable_testing()
add_library(skyhop_harness STATIC tests/harness.cpp)
target_include_directories(skyhop_harness PUBLIC tests)

# Pure: the simulation alone.
foreach(name geom_cases levels_cases physics_cases render_cases start_worlds moves touches runs laws)
  add_executable(${name}_test tests/${name}_test.cpp)
  target_link_libraries(${name}_test PRIVATE skyhop_harness hop_sim)
  add_test(NAME ${name} COMMAND ${name}_test)
endforeach()

# The vocabulary, without a window.
foreach(name cli_parser script_reader app)
  add_executable(${name}_test tests/${name}_test.cpp)
  target_link_libraries(${name}_test PRIVATE skyhop_harness game_app)
  add_test(NAME ${name} COMMAND ${name}_test)
endforeach()

# Through the real raylib: the box test needs no window, the screen tests need an X server.
add_executable(edge_test tests/edge_test.cpp)
target_link_libraries(edge_test PRIVATE skyhop_harness hop_screen)
add_test(NAME edge COMMAND edge_test)

add_executable(screen_test tests/screen_test.cpp)
target_link_libraries(screen_test PRIVATE skyhop_harness game_app)
target_compile_definitions(screen_test PRIVATE
  SKYHOP_TEST_ASSETS="${CMAKE_CURRENT_SOURCE_DIR}/../../assets"
  SKYHOP_TEST_EXE="$<TARGET_FILE:skyhop>")
add_dependencies(screen_test skyhop)
add_test(NAME screen COMMAND screen_test)
set_tests_properties(screen PROPERTIES SKIP_RETURN_CODE 77)
