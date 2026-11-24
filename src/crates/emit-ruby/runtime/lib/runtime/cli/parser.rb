require_relative "command"

module Runtime
  module CLI
    # A rule and its arguments, every parameter present.
    Invocation = Data.define(:command, :args)

    # Arguments to an Invocation, or the first problem from the left as a UsageError.
    class Parser
      HELP_FLAGS = ["--help", "-h"].freeze
      END_OF_FLAGS = "--"

      # `--help` or `-h` anywhere before the first `--`.
      def self.help?(args)
        args.take_while { |arg| arg != END_OF_FLAGS }.intersect?(HELP_FLAGS)
      end

      # `args` is not empty: its first item is the command, whatever it looks like.
      def self.parse(app, args)
        name, *rest = args
        command = app.find(name) or raise UsageError, "unknown command \"#{name}\""
        new(command).parse(rest)
      end

      def initialize(command)
        @command = command
        @given = {}
        @pending = command.positionals.dup
      end

      def parse(rest)
        queue = rest.dup
        flags_open = true
        until queue.empty?
          arg = queue.shift
          if flags_open && arg == END_OF_FLAGS then flags_open = false
          elsif flags_open && arg.start_with?("--") then flag(arg, queue)
          else
            positional(arg)
          end
        end
        raise UsageError, "missing #{@pending.first.shown}" unless @pending.empty?

        Invocation.new(command: @command, args: @command.defaults.merge(@given))
      end

      private

      def flag(shown, queue)
        flag = @command.flag(shown) or raise UsageError, "unknown flag #{shown}"
        return switch(flag) if flag.kind == :switch
        raise UsageError, "#{shown} needs a value" if queue.empty?

        valued(flag, queue.shift)
      end

      # A flag seen before is refused before its new value is looked at; only a Set repeats.
      def valued(flag, text)
        if flag.kind == :set
          @given[flag.name] = (@given.fetch(flag.name, Set[]) | [value(flag.type, text, flag.shown)]).freeze
        else
          raise UsageError, "#{flag.shown} given twice" if @given.key?(flag.name)

          @given[flag.name] = value(flag.type, text, flag.shown)
        end
      end

      def value(type, text, shown)
        value = type.parse(text)
        raise UsageError, "#{shown}: not a #{type.name}" if value.nil?

        value
      end

      def switch(flag)
        raise UsageError, "#{flag.shown} given twice" if @given.key?(flag.name)

        @given[flag.name] = true
      end

      def positional(text)
        param = @pending.shift or raise UsageError, "unexpected argument \"#{text}\""
        @given[param.name] = value(param.type, text, param.shown)
      end
    end
  end
end
