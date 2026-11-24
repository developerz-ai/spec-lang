# The `game` library's runtime for C++ (libs/game/real/cpp), included by the app's CMakeLists.txt: the four
# commands, the input script and the window loop. It uses the `cli` library's `out` handler and argument
# values, and draws through whatever library installs a screen.
add_library(lib_game STATIC
  lib/game/app.cpp
  lib/game/args.cpp
  lib/game/script.cpp
)
target_include_directories(lib_game PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(lib_game PUBLIC program lib_cli)
target_link_libraries(app PRIVATE lib_game)

# @bin(name): the executable's name.
if(DEFINED APP_NOTE_game_bin_name)
  set_target_properties(app PROPERTIES OUTPUT_NAME "${APP_NOTE_game_bin_name}")
endif()

# The runtime's own tests: the command line and the input script over the app's manifest, without a window.
add_executable(lib_game_test lib/game/tests/game_test.cpp)
target_link_libraries(lib_game_test PRIVATE lib_game program_blocked)
add_test(NAME lib_game COMMAND lib_game_test)
