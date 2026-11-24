# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: `bin/test`.
# The same in every generated project; change the printer, not this file.

require "etc"

module Spec
  module Testing
    # Runs every check and prints one JSON line per check, in declared order:
    # `{"check": <name>, "status": "pass" | "fail" | "blocked"}`, with `"why"` when it did not pass.
    # Checks run in child processes, `--jobs` at a time; the output does not depend on it.
    module Runner
      USAGE = "usage: bin/test [--seed N] [--runs N] [--traces N] [--steps N] [--prefix N] [--jobs N]"

      module_function

      def run(argv, out: $stdout, err: $stderr)
        suite = Testing.suite
        suite.configure(config(argv))
        outcomes = results(suite, suite.config.jobs || Etc.nprocessors)
        suite.checks.zip(outcomes) { |check, outcome| out.puts(line(check, outcome)) }
        out.flush
        outcomes.any? { |outcome| outcome.status == "fail" } ? 1 : 0
      rescue ArgumentError => e
        err.puts("error: #{e.message}", USAGE)
        2
      end

      def config(argv)
        config = Config.default
        argv.each_slice(2) do |flag, value|
          key = flag.to_s.delete_prefix("--").to_sym
          raise ArgumentError, "unexpected argument #{flag.inspect}" unless flag.start_with?("--") && config.members.include?(key)

          config[key] = Integer(value.to_s, 10)
        end
        config
      end

      def one(check)
        check.run.call
      rescue StandardError, SystemStackError => e
        Outcome.of(e)
      end

      def results(suite, jobs)
        return suite.checks.map { |check| one(check) } if jobs <= 1 || !Process.respond_to?(:fork)

        forked(suite.checks, jobs)
      end

      # Worker `w` of `jobs` runs checks w, w + jobs, ...; each writes its outcomes down its own pipe.
      def forked(checks, jobs)
        workers = Array.new([jobs, checks.size].min) do |worker|
          reader, writer = IO.pipe
          pid = fork do
            reader.close
            worker.step(checks.size - 1, jobs) { |i| Marshal.dump([i, one(checks[i]).to_h], writer) }
            writer.close
            exit!(0)
          end
          writer.close
          [pid, reader]
        end
        collect(workers, checks.size)
      end

      def collect(workers, count)
        outcomes = Array.new(count)
        workers.each do |pid, reader|
          until reader.eof?
            index, outcome = Marshal.load(reader) # rubocop:disable Security/MarshalLoad -- our own child
            outcomes[index] = Outcome.new(**outcome)
          end
          reader.close
          Process.wait(pid)
        end
        outcomes.map { |outcome| outcome || Outcome.new("fail", "E-RUNTIME", "the check's process ended early") }
      end

      def line(check, outcome)
        why = outcome.message ? %(, "why": #{json(outcome.message)}) : ""
        %({"check": #{json(check.name)}, "kind": #{json(check.kind.to_s)}, "status": #{json(outcome.status)}#{why}})
      end

      def json(text)
        escaped = text.to_s.scrub("?").gsub(/["\\\u0000-\u001F]/) do |c|
          case c
          when '"' then '\\"'
          when "\\" then "\\\\"
          else format("\\u%04x", c.ord)
          end
        end
        "\"#{escaped}\""
      end
    end
  end
end
