# frozen_string_literal: true

# Projected by hand from @store(file(".todo.json")): load before the rule, one lock around it, an atomic replace after it (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "errors"
require_relative "lock"
require_relative "decoder"

module Todo
  module Store
    # Where every state of the app lives between runs.
    class FileStore
      def initialize(path, lock_timeout: Lock::TIMEOUT)
        @path = path
        @lock_timeout = lock_timeout
      end

      # Runs the block on what was read, under the lock; the block gives back an outcome with a `world`.
      # :write holds the lock exclusively and commits that world; :read shares the lock and never writes.
      # `reads` picks how much of the store is built: :world, or :counts.
      def transact(access, reads: :world)
        Lock.hold("#{@path}.lock", exclusive: access == :write, timeout: @lock_timeout) do
          bytes = read
          outcome = yield decode(bytes, reads)
          commit(outcome.world, bytes) if access == :write
          outcome
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

      def decode(bytes, reads)
        return initial(reads) if bytes.nil?

        Decoder.public_send(reads, bytes)
      rescue Decoder::Malformed
        raise Unreadable, @path
      end

      # A missing store is the initial world.
      def initial(reads)
        return Board::Counts.new(open: 0, size: 0) if reads == :counts

        require_relative "../board/world"
        Board::World.initial
      end

      # The writer is loaded here, so a rule that only reads never loads it.
      def commit(world, bytes_read)
        require_relative "encoder"
        require_relative "atomic_file"
        bytes = Encoder.encode(world)
        # Bytes equal to those read are not written again: the replace would change nothing a reader can see.
        return if bytes == bytes_read

        AtomicFile.replace(@path, bytes)
      rescue Encoder::Full
        raise Full, @path
      rescue SystemCallError, IOError
        raise Unwritable, @path
      end
    end
  end
end
