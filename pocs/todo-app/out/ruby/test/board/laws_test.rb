# frozen_string_literal: true

# Projected by hand from law numbers_below_next, next_only_grows, others_untouched, clear_keeps_every_open_task, reads_change_nothing, refusal_changes_nothing (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "support/traces"

# Every law, after every step of every trace. A failure names the trace's seed.
class LawsTest < Minitest::Test
  TRACES = 250
  STEPS = 40
  SEED = Integer(ENV.fetch("TODO_SEED", "20261124"))
  CHANGES = %i[add done reopen rename remove clear].freeze
  NAMING = %i[done reopen rename remove].freeze
  READS = %i[list stats].freeze
  OPEN = Todo::Model::Status::OPEN

  def test_every_law_holds_after_every_step
    steps = 0
    refusals = 0
    TRACES.times do |index|
      seed = SEED + index
      traces = Todo::TestSupport::Traces.new(seed)
      start = traces.world
      numbers_below_next(start, "seed #{seed}, start")
      traces.run(start, STEPS) do |step|
        where = "seed #{seed}, step #{steps % STEPS}: #{step.event.rule} #{step.event.args}"
        snapshot = Marshal.load(Marshal.dump(step.before))
        check(step, where)

        assert_equal snapshot, step.before, "#{where}: the world before was changed in place"
        steps += 1
        refusals += 1 if step.refused?
      end
    end

    assert_equal TRACES * STEPS, steps
    assert_operator refusals, :>, TRACES, "the traces must exercise refusals"
  end

  private

  def check(step, where)
    numbers_below_next(step.after, where)
    next_only_grows(step, where)
    others_untouched(step, where) if NAMING.include?(step.event.rule)
    clear_keeps_every_open_task(step, where) if step.event.rule == :clear
    reads_change_nothing(step, where) if READS.include?(step.event.rule)
    refusal_changes_nothing(step, where) if CHANGES.include?(step.event.rule)
  end

  # t.states.all(w => w.tasks.keys.all(it < w.next_no))
  def numbers_below_next(world, where)
    assert world.tasks.keys.all? { |no| no < world.next_no }, "numbers_below_next: #{where}"
  end

  # t.steps.all(it.after.next_no >= it.before.next_no)
  def next_only_grows(step, where)
    assert_operator step.after.next_no, :>=, step.before.next_no, "next_only_grows: #{where}"
  end

  # s.event.no != k implies s.after.tasks.get(k) == s.before.tasks.get(k), for every k either world knows
  # and a few neither does.
  def others_untouched(step, where)
    candidates = step.before.tasks.keys | step.after.tasks.keys | [step.before.next_no, step.before.next_no + 1]
    candidates.each do |k|
      next if step.event.args.fetch(:no) == k

      message = "others_untouched(k: #{k}): #{where}"
      before = step.before.tasks[k]
      before.nil? ? assert_nil(step.after.tasks[k], message) : assert_equal(before, step.after.tasks[k], message)
    end
  end

  def clear_keeps_every_open_task(step, where)
    assert step.after.tasks.each_value.all? { |task| task.status == OPEN }, "clear_keeps_every_open_task: #{where}"
    assert_equal step.before.tasks.each_value.count { |task| task.status == OPEN }, step.after.tasks.size,
                 "clear_keeps_every_open_task: #{where}"
  end

  # t.steps.all(it.after == it.before)
  def reads_change_nothing(step, where)
    assert_equal step.before, step.after, "reads_change_nothing: #{where}"
  end

  # s.reply.is(fail(_)) implies s.after == s.before
  def refusal_changes_nothing(step, where)
    assert_equal step.before, step.after, "refusal_changes_nothing: #{where}" if step.refused?
  end
end
