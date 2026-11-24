# frozen_string_literal: true

# Projected by hand from the parameters and @uses of rule add, list, done, reopen, rename, remove, clear, stats (src/todo/board.slg) in the order of app TodoRuby (src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "command"
require_relative "types"

module Todo
  module CLI
    # The per-command parameter table. Rules are named, not referenced, so that reading the table loads no rule.
    module Commands
      def self.command(name, access:, reads: :world, positionals: {}, flags: [])
        Command.new(name:, rule: name.to_sym, access:, reads:, flags:,
                    positionals: positionals.map { |param, type| Positional.new(name: param, type:) })
      end

      def self.switch(name) = Flag.new(name:, kind: :switch, type: nil, default: false)

      def self.once(name, type, default) = Flag.new(name:, kind: :once, type:, default:)

      def self.set(name, type) = Flag.new(name:, kind: :set, type:, default: Set[])

      ALL = Ractor.make_shareable(
        [
          command("add", access: :write, positionals: { title: Types::Title },
                         flags: [once(:priority, Types::Priority, Model::Priority::NORMAL), set(:tags, Types::Tag)]),
          command("list", access: :read, flags: [switch(:all), once(:tag, Types::Tag, nil)]),
          command("done", access: :write, positionals: { no: Types::TaskNo }),
          command("reopen", access: :write, positionals: { no: Types::TaskNo }),
          command("rename", access: :write, positionals: { no: Types::TaskNo, title: Types::Title }),
          command("remove", access: :write, positionals: { no: Types::TaskNo }),
          command("clear", access: :write),
          command("stats", access: :read, reads: :counts)
        ]
      )
      BY_NAME = ALL.to_h { |command| [command.name, command] }.freeze

      def self.find(name) = BY_NAME[name]
    end
  end
end
