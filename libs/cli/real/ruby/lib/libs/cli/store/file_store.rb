# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): load, lock, atomic replace.
# The same in every app that uses `cli`; change the library, not this file.

require_relative "errors"
require_relative "lock"
require_relative "schema"

module Libs
  module Cli
    module Store
      # Where every state of the app lives between runs: load before the rule, one lock around it, an atomic
      # replace after it.
      class FileStore
        def initialize(path, schema, lock_timeout: Lock::TIMEOUT)
          @path = path
          @schema = schema
          @lock_timeout = lock_timeout
        end

        # Runs the block on the world that was read, under the lock; the block gives the world to keep.
        # A writer holds the lock exclusively and commits that world; a reader shares it and never writes.
        def transact(writes)
          Lock.hold("#{@path}.lock", exclusive: writes, timeout: @lock_timeout) do
            bytes = read
            world = yield decode(bytes)
            commit(world, bytes) if writes
          end
        rescue Lock::Unavailable
          raise Unwritable, @path
        rescue Lock::Busy
          raise Contended
        end

        private

        # The store's bytes, tagged UTF-8 and not yet checked, or nil when there is no store yet.
        def read
          File.binread(@path).force_encoding(Encoding::UTF_8)
        rescue Errno::ENOENT
          nil
        rescue SystemCallError
          raise Unreadable, @path
        end

        # A missing store is the initial world.
        def decode(bytes)
          bytes.nil? ? @schema.initial : @schema.decode(bytes)
        rescue Malformed
          raise Unreadable, @path
        end

        # The writer is loaded here, so a rule that only reads never loads it.
        def commit(world, bytes_read)
          require_relative "atomic_file"
          bytes = @schema.encode(world)
          # Bytes equal to those read are not written again: the replace would change nothing a reader can see.
          return if bytes == bytes_read

          AtomicFile.replace(@path, bytes)
        rescue TooLarge
          raise Full, @path
        rescue SystemCallError, IOError, Unkeepable
          raise Unwritable, @path
        end
      end
    end
  end
end
