# Projected by hand from app TodoCpp, @target(cpp), @bin("todo") (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
cmake_minimum_required(VERSION 3.28)
project(todo LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

add_compile_options(-Wall -Wextra -Wpedantic -Werror)

# Release: whole-program optimisation, one static binary (@why "the board as one static binary").
include(CheckIPOSupported)
check_ipo_supported(RESULT ipo_ok OUTPUT ipo_message LANGUAGES CXX)
if(ipo_ok)
  set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)
endif()

# todo.model, todo.render, todo.board: pure, no I/O. Nothing here links the store or the cli.
add_library(todo_domain STATIC
  src/model/unicode.cpp
  src/model/text.cpp
  src/render/render.cpp
  src/board/add.cpp
  src/board/list.cpp
  src/board/done_reopen.cpp
  src/board/rename.cpp
  src/board/remove.cpp
  src/board/clear.cpp
  src/board/stats.cpp
)
target_include_directories(todo_domain PUBLIC include)

# @store(file(".todo.json")): codec, lock, atomic file.
add_library(todo_store STATIC
  src/store/json_lexer.cpp
  src/store/decode.cpp
  src/store/encode.cpp
  src/store/lock.cpp
  src/store/file.cpp
)
target_link_libraries(todo_store PUBLIC todo_domain)

# The cli vocabulary: parameter table, parser, help, dispatch.
add_library(todo_cli STATIC
  src/cli/parser.cpp
  src/cli/dispatch.cpp
)
target_link_libraries(todo_cli PUBLIC todo_domain)

add_library(todo_app STATIC src/app/program.cpp)
target_link_libraries(todo_app PUBLIC todo_cli todo_store)

add_executable(todo src/app/main.cpp)
target_link_libraries(todo PRIVATE todo_app)
target_link_options(todo PRIVATE -static $<$<CONFIG:Release>:-s>)

# Tests: one executable per spec concern, a small harness, no framework.
enable_testing()
add_library(todo_harness STATIC tests/harness.cpp)
target_include_directories(todo_harness PUBLIC tests)

foreach(name model_cases render_cases scenarios laws store_codec store_files cli_parser program)
  add_executable(${name}_test tests/${name}_test.cpp)
  target_link_libraries(${name}_test PRIVATE todo_harness todo_app)
  add_test(NAME ${name} COMMAND ${name}_test)
endforeach()
