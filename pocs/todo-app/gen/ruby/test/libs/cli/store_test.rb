# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): tests of the store, on the app it was built into.
# The same in every app that uses `cli`; change the library, not this file.

require "minitest/autorun"
require "tmpdir"
require "app"
require "libs/cli/store/file_store"
require "libs/cli/store/atomic_file"

module Libs
  module Cli
    module Store
      class StoreTest < Minitest::Test
        def schema = Schema.new(PROGRAM, App::MANIFEST)

        def test_the_initial_world_survives_the_store
          bytes = schema.encode(schema.initial)
          assert_equal schema.initial, schema.decode(bytes)
          assert bytes.start_with?('{"version":1')
          assert bytes.end_with?("}\n")
          refute_match(/[ \t]/, bytes.gsub(/"(?:[^"\\]|\\.)*"/, ""))
        end

        def test_a_store_with_another_version_or_another_key_is_malformed
          bytes = schema.encode(schema.initial)
          assert_raises(Malformed) { schema.decode(bytes.sub('"version":1', '"version":2')) }
          assert_raises(Malformed) { schema.decode(bytes.sub("}\n", ",\"extra\":1}\n")) }
          assert_raises(Malformed) { schema.decode("[]\n") }
        end

        def test_the_reader_refuses_what_json_parse_lets_through
          assert_raises(JsonReader::Invalid) { JsonReader.parse('{"a":1,"a":2}') }
          assert_raises(JsonReader::Invalid) { JsonReader.parse('"\ud800"') }
          assert_raises(JsonReader::Invalid) { JsonReader.parse("1 2") }
          assert_kind_of JsonReader::Real, JsonReader.parse("1.0")
          assert_equal 9_007_199_254_740_993, JsonReader.parse("9007199254740993")
          assert_equal "\u{1F600}", JsonReader.parse('"😀"')
        end

        def test_an_int_past_two_to_the_53_is_too_large_to_keep
          assert_equal "9007199254740992", Codec.integer(2**53)
          assert_raises(TooLarge) { Codec.integer((2**53) + 1) }
          assert_raises(Malformed) { Codec.decode(Spec::Type::INT, (2**53) + 1) }
          assert_raises(Malformed) { Codec.decode(Spec::Type::INT, JsonReader.parse("1e0")) }
        end

        def test_text_escapes_only_what_json_needs
          out = +""
          Codec.quote("a\"b\\c é \u{1F600}", out)
          assert_equal "\"a\\\"b\\\\c é \u{1F600}\"", out
        end

        def test_a_replace_is_whole_and_leaves_no_temporary_file
          Dir.mktmpdir do |dir|
            path = File.join(dir, "store")
            AtomicFile.replace(path, "one\n")
            AtomicFile.replace(path, "two\n")
            assert_equal "two\n", File.read(path)
            assert_equal ["store"], Dir.children(dir)
          end
        end

        def test_a_failed_replace_keeps_the_old_bytes_and_no_temporary_file
          Dir.mktmpdir do |dir|
            path = File.join(dir, "store")
            Dir.mkdir(path) # a rename over a directory fails
            assert_raises(SystemCallError) { AtomicFile.replace(path, "new\n") }
            assert_equal ["store"], Dir.children(dir)
          end
        end

        def test_a_held_lock_is_busy_when_the_wait_is_over
          Dir.mktmpdir do |dir|
            path = File.join(dir, "store.lock")
            Lock.hold(path, exclusive: true) do
              assert_raises(Lock::Busy) { Lock.hold(path, exclusive: false, timeout: 0.05) { flunk } }
            end
            Lock.hold(path, exclusive: false) { Lock.hold(path, exclusive: false, timeout: 0.05) { pass } }
          end
        end

        def test_a_reader_never_creates_the_store
          Dir.mktmpdir do |dir|
            path = File.join(dir, "store")
            FileStore.new(path, schema).transact(false) { |world| world }
            assert_equal ["store.lock"], Dir.children(dir)
            FileStore.new(path, schema).transact(true) { |world| world }
            assert_equal %w[store store.lock], Dir.children(dir).sort
          end
        end

        def test_a_lock_that_cannot_be_made_is_unwritable
          error = assert_raises(Unwritable) { FileStore.new("/no/such/dir/store", schema).transact(true) { |w| w } }
          assert_equal "store /no/such/dir/store: unwritable", error.message
        end
      end
    end
  end
end
