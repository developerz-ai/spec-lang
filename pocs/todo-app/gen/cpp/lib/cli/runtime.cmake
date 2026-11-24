# The `cli` library's runtime for C++ (libs/cli/real/cpp), included by the app's CMakeLists.txt: arguments from
# the manifest, the JSON store with its lock and atomic replace, and the `out` handler.
add_library(lib_cli STATIC
  lib/cli/args.cpp
  lib/cli/codec.cpp
  lib/cli/file.cpp
  lib/cli/json.cpp
  lib/cli/out.cpp
  lib/cli/run.cpp
)
target_include_directories(lib_cli PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(lib_cli PUBLIC program)
target_link_libraries(app PRIVATE lib_cli)

# @bin(name): the executable's name.
if(DEFINED APP_NOTE_cli_bin_name)
  set_target_properties(app PROPERTIES OUTPUT_NAME "${APP_NOTE_cli_bin_name}")
endif()

# The runtime's own tests: the store's format and its failure paths over the app's manifest.
add_executable(lib_cli_test lib/cli/tests/store_test.cpp)
target_link_libraries(lib_cli_test PRIVATE lib_cli program_blocked)
add_test(NAME lib_cli COMMAND lib_cli_test)
