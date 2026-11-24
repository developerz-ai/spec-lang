# frozen_string_literal: true

# Runtime shim of the ruby backend of spec-lang build: failure paths: a failed or interrupted write leaves the old store or the new and no tmp file; locks are released. The same in every generated project; change the backend, not this file.

require "minitest/autorun"
require "tmpdir"
require "rbconfig"
require "runtime/store/atomic_file"
require "runtime/store/lock"

class RuntimeFailurePathsTest < Minitest::Test
  AtomicFile = Runtime::Store::AtomicFile
  Lock = Runtime::Store::Lock

  def test_a_replace_that_fails_leaves_the_old_store_and_no_tmp_file
    Dir.mktmpdir("spec-runtime-") do |dir|
      # A directory cannot be renamed over, so the last step fails after the tmp file was written.
      path = File.join(dir, "store")
      Dir.mkdir(path)
      File.write(File.join(path, "kept"), "old")

      assert_raises(SystemCallError) { AtomicFile.replace(path, "new\n") }
      assert_equal ["store"], Dir.children(dir)
      assert_equal "old", File.read(File.join(path, "kept"))
    end
  end

  def test_a_replace_into_a_missing_directory_fails_and_leaves_nothing
    Dir.mktmpdir("spec-runtime-") do |dir|
      assert_raises(SystemCallError) { AtomicFile.replace(File.join(dir, "none", "store"), "new\n") }
      assert_empty Dir.children(dir)
    end
  end

  def test_a_lock_is_released_on_every_way_out
    Dir.mktmpdir("spec-runtime-") do |dir|
      path = File.join(dir, "store.lock")
      assert_raises(RuntimeError) { Lock.hold(path, exclusive: true, timeout: 0) { raise "the rule had a bug" } }
      # Released by the failure above: the next taker gets it at once, and a third is refused while it holds.
      Lock.hold(path, exclusive: true, timeout: 0) do
        assert_raises(Lock::Busy) { Lock.hold(path, exclusive: true, timeout: 0) { flunk "two holders" } }
      end

      assert_equal ["store.lock"], Dir.children(dir)
    end
  end

  def test_an_interrupt_during_a_write_leaves_the_old_store_or_the_new_and_no_tmp_file
    old = "#{"o" * (1 << 16)}\n"
    fresh = "#{"n" * (1 << 16)}\n"
    lib = File.expand_path("../../lib", __dir__)
    script = 'require "runtime/store/atomic_file"; $stdout.write("ready\n"); $stdout.flush; ' \
             "400.times { Runtime::Store::AtomicFile.replace(ARGV[0], ARGV[1]) }"
    8.times do |round|
      Dir.mktmpdir("spec-runtime-") do |dir|
        path = File.join(dir, "store")
        File.write(path, old)
        IO.popen([RbConfig.ruby, "--disable-gems", "-I", lib, "-e", script, path, fresh], err: File::NULL) do |child|
          child.gets
          sleep(round / 1000.0)
          Process.kill("INT", child.pid)
        end

        assert_includes [old, fresh], File.read(path)
        assert_equal ["store"], Dir.children(dir)
      end
    end
  end
end
