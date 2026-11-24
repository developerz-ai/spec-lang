# frozen_string_literal: true

# Projected by hand from the commands the game vocabulary derives for app SkyhopRuby (docs/game-vocabulary.md, src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module CLI
    HELP = <<~TEXT
      usage: skyhop <command> [options]

      commands:
        play [--level <n>]
        run [--level <n>]
        draw [--level <n>] [--at <tick>]
        shot [--level <n>] [--at <tick>] [--out <file.png>]

      play opens a window: arrows or WASD to run, space to jump, Esc to quit.
      run, draw and shot read a script of controls on standard input.
    TEXT
  end
end
