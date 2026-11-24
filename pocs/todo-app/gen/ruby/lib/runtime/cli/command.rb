# frozen_string_literal: true

# Runtime shim of the ruby backend of spec-lang build: command table rows, argument types, help, exit codes. The same in every generated project; change the backend, not this file.

module Runtime
  # The command line of an app: arguments in, lines and an exit code out.
  module CLI
    # The arguments are not a call of any rule. The message is the line after "error: ".
    class UsageError < StandardError; end

    module ExitCode
      OK = 0
      REFUSED = 1
      USAGE = 2
      STORE = 3
    end

    # One argument's text to a value of the type, or nil when the text is not of the type.
    Type = Data.define(:name, :placeholder, :parser) do
      def parse(text) = parser.call(text)
    end

    DIGITS = /\A-?[0-9]+\z/

    # `Int where ...`: ASCII digits, leading zeros allowed, any length since Integer is exact; then the claim.
    def self.int_type(type)
      parser = lambda do |text|
        next unless text.valid_encoding? && text.match?(DIGITS)

        value = Integer(text, 10)
        value if type.valid?(value)
      end
      Type.new(name: type::NAME, placeholder: "<#{type::NAME.downcase}>", parser:)
    end

    # `Text where ...`: the argument as given, then the claim.
    def self.text_type(type)
      Type.new(name: type::NAME, placeholder: "<#{type::NAME.downcase}>", parser: ->(text) { text if type.valid?(text) })
    end

    # A sum type with no data: its variant names in lower case.
    def self.sum_type(name, all)
      variants = all.to_h { |variant| [variant.name.downcase, variant] }.freeze
      Type.new(name:, placeholder: variants.keys.join("|").freeze, parser: ->(text) { variants[text] })
    end

    # A parameter without a default: a positional argument, shown `<name>`.
    Positional = Data.define(:name, :type) do
      def shown = "<#{name}>"
    end

    # A parameter with a default. `kind` is :switch (Bool = false), :once (one value at most)
    # or :set (Set[T], repeatable).
    Flag = Data.define(:name, :kind, :type, :default) do
      def shown = "--#{name}"

      def usage
        case kind
        when :switch then "[#{shown}]"
        when :once then "[#{shown} #{type.placeholder}]"
        when :set then "[#{shown} #{type.placeholder}]..."
        end
      end
    end

    # One `@on(cli(name))` rule. `access` is :write or :read, from the rule's write set. `load` is the
    # file that holds the rule and `rule` calls it, so reading the table loads no rule.
    Command = Data.define(:name, :access, :load, :rule, :positionals, :flags) do
      def flag(shown) = flags.find { |flag| flag.shown == shown }

      def defaults = flags.to_h { |flag| [flag.name, flag.default] }

      def usage = [name, *positionals.map(&:shown), *flags.map(&:usage)].join(" ")
    end

    def self.positional(name, type) = Positional.new(name:, type:)

    def self.switch(name) = Flag.new(name:, kind: :switch, type: nil, default: false)

    def self.once(name, type, default) = Flag.new(name:, kind: :once, type:, default:)

    def self.set(name, type) = Flag.new(name:, kind: :set, type:, default: Set[].freeze)

    # `--help`, derived from the command table: one line per command, in the app's order.
    def self.help(bin, commands)
      "usage: #{bin} <command> [args]\n\ncommands:\n#{commands.map { |command| "  #{command.usage}\n" }.join}".freeze
    end

    # Everything the `app` declaration fixes. `store` gives the store's codec and first world when
    # called, so help and usage errors never load them.
    App = Data.define(:bin, :commands, :help, :store_file, :store_env, :store) do
      def find(name) = commands.find { |command| command.name == name }

      # The store path as configured, never resolved: errors print it this way.
      def store_path(env)
        configured = env[store_env]
        configured.nil? || configured.empty? ? store_file : configured
      end
    end

    def self.app(bin:, commands:, store_file:, store_env:, store:)
      App.new(bin:, commands: commands.freeze, help: help(bin, commands), store_file:, store_env:, store:)
    end
  end
end
