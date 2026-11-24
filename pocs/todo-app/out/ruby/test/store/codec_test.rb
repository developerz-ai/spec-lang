# frozen_string_literal: true

# Projected by hand from the store format: bytes written, accepted on read (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "support/traces"

class CodecTest < Minitest::Test
  Decoder = Todo::Store::Decoder
  Encoder = Todo::Store::Encoder
  World = Todo::Board::World
  Task = Todo::Model::Task

  CONTRACT_WORLD = World.new(
    tasks: {
      2 => Task.new(no: 2, title: 'bread "fresh"', status: :Done, priority: :High, tags: Set["shop", "home"]),
      1 => Task.new(no: 1, title: "milk")
    },
    next_no: 3
  )
  CONTRACT_BYTES = <<~'JSON'
    {"version":1,"next_no":3,"tasks":[{"no":1,"title":"milk","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"bread \"fresh\"","status":"Done","priority":"High","tags":["home","shop"]}]}
  JSON

  def test_canonical_bytes_are_the_contracts
    assert_equal CONTRACT_BYTES, Encoder.encode(CONTRACT_WORLD)
    assert_equal %({"version":1,"next_no":1,"tasks":[]}\n), Encoder.encode(World.initial)
  end

  def test_only_quote_and_backslash_are_escaped
    world = World.new(tasks: { 1 => Task.new(no: 1, title: "a\"b\\c café \u{1F600} \u2027\u0084 </script> '") }, next_no: 2)

    assert_equal %({"version":1,"next_no":2,"tasks":[{"no":1,"title":"a\\"b\\\\c café \u{1F600} \u2027\u0084 </script> '","status":"Open","priority":"Normal","tags":[]}]}\n),
                 Encoder.encode(world)
    assert_equal Encoding::UTF_8, Encoder.encode(world).encoding
  end

  def test_round_trip
    assert_equal CONTRACT_WORLD, Decoder.world(CONTRACT_BYTES)
    150.times do |seed|
      traces = Todo::TestSupport::Traces.new(seed)
      traces.run(traces.world, 10) do |step|
        bytes = Encoder.encode(step.after)

        assert_equal step.after, Decoder.world(bytes), "seed #{seed}"
        assert_equal bytes, Encoder.encode(Decoder.world(bytes)), "seed #{seed}"
        assert_equal step.after.counts, Decoder.counts(bytes), "seed #{seed}"
      end
    end
  end

  def test_reads_any_valid_json_spelling
    text = '{ "tasks" : [ {"tags":["b","a"],"priority":"High","status":"Done","title":"café 😀","no":2}, ' \
           '{"no":1,"title":"x","status":"Open","priority":"Low","tags":[]} ], "next_no" : 7, "version" : 1 }'
    world = Decoder.world(text)

    assert_equal 7, world.next_no
    assert_equal Task.new(no: 2, title: "café \u{1F600}", status: :Done, priority: :High, tags: Set["a", "b"]), world.tasks[2]
    assert_equal Task.new(no: 1, title: "x", priority: :Low), world.tasks[1]
    assert_equal Todo::Board::Counts.new(open: 1, size: 2), Decoder.counts(text)
  end

  def test_the_bounds_of_next_no
    assert_equal 2**53, Decoder.world(%({"version":1,"next_no":#{2**53},"tasks":[]})).next_no
    assert_equal %({"version":1,"next_no":#{2**53},"tasks":[]}\n), Encoder.encode(World.new(tasks: {}, next_no: 2**53))
    assert_raises(Encoder::Full) { Encoder.encode(World.new(tasks: {}, next_no: (2**53) + 1)) }
  end

  def self.task(fields = {}, extra = "")
    defaults = { no: "1", title: '"a"', status: '"Open"', priority: '"Normal"', tags: "[]" }
    members = defaults.merge(fields).compact.map { |key, json| %("#{key}":#{json}) }.join(",")
    %({"version":1,"next_no":2,"tasks":[{#{members}#{extra}}]})
  end

  UNREADABLE = {
    "blank" => "", "not JSON" => "todo", "an array" => "[1]", "empty object" => "{}",
    "trailing garbage" => '{"version":1,"next_no":1,"tasks":[]} x',
    "invalid UTF-8" => task(title: "\"\xFF\""),
    "version 2" => '{"version":2,"next_no":1,"tasks":[]}',
    "version 1.0" => '{"version":1.0,"next_no":1,"tasks":[]}',
    "version a string" => '{"version":"1","next_no":1,"tasks":[]}',
    "version true" => '{"version":true,"next_no":1,"tasks":[]}',
    "next_no zero" => '{"version":1,"next_no":0,"tasks":[]}',
    "next_no negative" => '{"version":1,"next_no":-1,"tasks":[]}',
    "next_no 1.0" => '{"version":1,"next_no":1.0,"tasks":[]}',
    "next_no 1e0" => '{"version":1,"next_no":1e0,"tasks":[]}',
    "next_no a string" => '{"version":1,"next_no":"1","tasks":[]}',
    "next_no null" => '{"version":1,"next_no":null,"tasks":[]}',
    "next_no above 2^53" => '{"version":1,"next_no":9007199254740993,"tasks":[]}',
    "tasks an object" => '{"version":1,"next_no":1,"tasks":{}}',
    "tasks null" => '{"version":1,"next_no":1,"tasks":null}',
    "extra top-level key" => '{"version":1,"next_no":1,"tasks":[],"x":0}',
    "missing key" => '{"version":1,"next_no":1}',
    "duplicate key" => '{"version":1,"next_no":1,"next_no":1,"tasks":[]}',
    "a task that is not an object" => '{"version":1,"next_no":2,"tasks":[1]}',
    "duplicate key in a task" => task({}, ',"no":1'),
    "extra task key" => task({}, ',"due":null'),
    "missing task key" => task(tags: nil),
    "no not below next_no" => task(no: "2"),
    "no zero" => task(no: "0"),
    "no 1.0" => task(no: "1.0"),
    "no a string" => task(no: '"1"'),
    "no twice" => '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},' \
                  '{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}',
    "blank title" => task(title: '" "'),
    "title with a tab" => task(title: '"a\tb"'),
    "title with a line separator" => task(title: %("a\u2028b")),
    "title with an escaped next line" => task(title: '"a\u0085b"'),
    "title too long" => task(title: %("#{"x" * 201}")),
    "title a number" => task(title: "1"),
    "title a lone surrogate" => task(title: '"\ud800"'),
    "status in lower case" => task(status: '"open"'),
    "status null" => task(status: "null"),
    "priority unknown" => task(priority: '"Urgent"'),
    "priority a number" => task(priority: "1"),
    "tags not an array" => task(tags: '"home"'),
    "bad tag" => task(tags: '["Home"]'),
    "tag starting with a dash" => task(tags: '["-x"]'),
    "tag not a string" => task(tags: "[1]"),
    "tag twice" => task(tags: '["a","a"]'),
    "tag twice, written two ways" => task(tags: '["a","a"]')
  }.freeze

  def test_each_unreadable_shape
    UNREADABLE.each do |name, text|
      assert_raises(Decoder::Malformed, "world: #{name}") { Decoder.world(text) }
      assert_raises(Decoder::Malformed, "counts: #{name}") { Decoder.counts(text) }
    end
  end

  def test_the_unreadable_table_is_built_from_a_readable_task
    assert_equal 1, Decoder.world(self.class.task).tasks.size
  end
end
