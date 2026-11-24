# frozen_string_literal: true

# Projected by hand from Trace[Change], Trace[board.list | board.stats] and their steps (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module TestSupport
    # Seeded random traces over the rules of todo.board. A trace starts from the initial world or from
    # any world the types and laws allow; every step keeps event, reply, before and after.
    class Traces
      Event = Data.define(:rule, :args)
      # `reply` is the rule's reply, or the Refusal it failed with.
      Step = Data.define(:event, :reply, :before, :after) do
        def refused? = reply.is_a?(Board::Refusal)
      end

      TITLES = ["milk", "ship it", "café ☕", "  padded  ", "\"quoted\" back\\slash", "\u{1F600}" * 3, "x" * 200].freeze
      TAGS = %w[home work shop deep-work a b 7].freeze
      RULES = %i[add add add done done reopen rename remove clear list list stats].freeze

      def initialize(seed)
        @random = Random.new(seed)
      end

      def world
        return Board::World.initial if @random.rand(2).zero?

        next_no = @random.rand(1..12)
        numbers = (1...next_no).select { @random.rand(3).positive? }
        Board::World.new(tasks: numbers.to_h { |no| [no, task(no)] }, next_no:)
      end

      # Runs `length` random commands from `world`, yielding each step as it happens.
      def run(world, length)
        length.times do
          event = event(world)
          step = step(world, event)
          yield step
          world = step.after
        end
      end

      private

      def step(before, event)
        outcome = Board.public_send(event.rule, before, **event.args)
        Step.new(event:, reply: outcome.reply, before:, after: outcome.world)
      rescue Board::Refusal => e
        Step.new(event:, reply: e, before:, after: before)
      end

      def event(world)
        rule = RULES.sample(random: @random)
        Event.new(rule:, args: args(rule, world))
      end

      def args(rule, world)
        case rule
        when :add then { title: title, priority: priority, tags: tags }
        when :list then { all: @random.rand(2).zero?, tag: @random.rand(2).zero? ? nil : TAGS.sample(random: @random) }
        when :rename then { no: no(world), title: title }
        when :done, :reopen, :remove then { no: no(world) }
        else {}
        end
      end

      # Mostly a task that exists, sometimes one that does not.
      def no(world)
        known = world.tasks.keys
        return known.sample(random: @random) if known.any? && @random.rand(5).positive?

        @random.rand(1..(world.next_no + 3))
      end

      def task(no)
        Model::Task.new(no:, title: title, status: Model::Status::ALL.sample(random: @random), priority: priority, tags: tags)
      end

      def title = TITLES.sample(random: @random)

      def priority = Model::Priority::ALL.sample(random: @random)

      def tags = TAGS.sample(@random.rand(0..3), random: @random).to_set
    end
  end
end
