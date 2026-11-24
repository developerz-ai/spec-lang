# frozen_string_literal: true

# Projected by hand from app SkyhopRuby(start: world.start, tick: world.step, input: screen.read_pad, frame: screen.present, dump: render.dump, report: world.summary) (src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "script"
require_relative "world"

module Skyhop
  # The four commands the game vocabulary derives: run, draw, shot, play.
  module App
    module_function

    def run(command, stdin:, out:, err:, env:)
      return play(command, out:, err:, env:) if command.name == "play"

      holds = Script.read(stdin.read)
      case command.name
      when "run" then replay(command, holds, out)
      when "draw" then out.write(lines(Render.dump(played(command, holds)))) && CLI::OK
      else
        require_relative "app/shot"
        Shot.call(command, played(command, holds), out:, err:, env:)
      end
    rescue Script::Invalid => e
      err.write("error: #{e.message}\n")
      CLI::USAGE
    end

    def lines(list) = list.map { "#{it}\n" }.join

    # Every tick of the script, while the game is playing; then the summary.
    def replay(command, holds, out)
      started = World.start(command.level)
      text = +lines(started.lines)
      world = started.world
      holds.each do |hold|
        hold.ticks.times do
          break unless world.mode == :Playing

          outcome = world.step(hold.pad)
          text << lines(outcome.lines)
          world = outcome.world
        end
      end
      out.write(text, world.summary, "\n")
      CLI::OK
    end

    # Silent play up to tick `at` (nil: the end of the script), or until the script or the game ends.
    def played(command, holds)
      world = World.start(command.level).world
      holds.each do |hold|
        hold.ticks.times do
          return world if world.mode != :Playing || (command.at && world.ticks >= command.at)

          world = world.step(hold.pad).world
        end
      end
      world
    end

    def play(command, out:, err:, env:)
      require_relative "app/play"
      Play.call(command, out:, err:, env:)
    end
  end
end

require_relative "render"
require_relative "cli"
