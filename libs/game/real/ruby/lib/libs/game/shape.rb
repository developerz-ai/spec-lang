# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): the app's six declarations, and what is derived from them.
# The same in every app with the game shape; change the library, not this file.

module Libs
  module Game
    # `app A(start: .., tick: .., input: .., frame: .., dump: .., report: ..)`, `@bin`, `@window` and the tick
    # rate, read from the manifest.
    class Shape
      attr_reader :program, :bin, :start, :tick, :input, :frame, :dump, :report

      def initialize(program, app)
        @program = program
        @app = app
        @bin = app.note("bin")["name"]
        @start, @tick, @frame = %w[start tick frame].map { |field| program.rules.fetch(named(field)) }
        @input, @dump, @report = %w[input dump report].map { |field| program.fns.fetch(named(field)) }
      end

      # `@window(width, height, title)`.
      def window = @app.note("window")

      # Ticks per second, from `@on(tick(60Hz))`.
      def rate = tick.triggers.find { |note| note.name == "tick" }["rate"].value

      # The start rule's one parameter, if it has one: `--level <n>`.
      def level = start.params.first

      # The tick rule's one parameter: a record of Bool fields, the controls of one tick.
      def pad = tick.params.first.type

      # One key per control: the field's first letter in upper case.
      def keys = @keys ||= pad.field_names.to_h { |name| [name[0].upcase, name] }

      # The flags of each command, in the order the help shows them.
      def flags
        first = level ? [level.name] : []
        { "play" => first, "run" => first, "draw" => first + ["at"], "shot" => first + %w[at out] }
      end

      HINTS = { "at" => "<tick>", "out" => "<file.png>" }.freeze

      def help
        lines = flags.map do |name, list|
          ["  #{name}", *list.map { |flag| "[--#{flag} #{HINTS.fetch(flag, "<n>")}]" }].join(" ")
        end
        controls = input.why ? "#{input.why}, " : ""
        <<~TEXT
          usage: #{bin} <command> [options]

          commands:
          #{lines.join("\n")}

          play opens a window: #{controls}Esc to quit.
          run, draw and shot read a script of controls on standard input.
        TEXT
      end

      private

      def named(field) = @app.params.fetch(field).first
    end
  end
end
