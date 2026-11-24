require_relative "errors"
require_relative "lock"
require_relative "codec"

module Runtime
  module Store
    # Where every state of the app lives between runs: load before the rule, one lock around it,
    # an atomic replace after it.
    class FileStore
      def initialize(path, codec:, initial:, lock_timeout: Lock::TIMEOUT)
        @path = path
        @codec = codec
        @initial = initial
        @lock_timeout = lock_timeout
      end

      # Runs the block on the world that was read, under the lock; the block gives back an outcome with a `world`.
      # :write holds the lock exclusively and commits that world; :read shares the lock and never writes.
      def transact(access)
        Lock.hold("#{@path}.lock", exclusive: access == :write, timeout: @lock_timeout) do
          bytes = read
          outcome = yield decode(bytes)
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

      # A missing store is the initial world.
      def decode(bytes)
        # The world is built here for this one rule call, so the rule may draft its maps in place.
        bytes.nil? ? @initial : Runtime.adopting { @codec.decode(bytes) }
      rescue Codec::Malformed
        raise Unreadable, @path
      end

      # The writer is loaded here, so a rule that only reads never loads it.
      def commit(world, bytes_read)
        require_relative "atomic_file"
        bytes = @codec.encode(world, capacity: (bytes_read&.bytesize || 0) + 256)
        # Bytes equal to those read are not written again: the replace would change nothing a reader can see.
        return if bytes == bytes_read

        AtomicFile.replace(@path, bytes)
      rescue Codec::Full
        raise Full, @path
      rescue SystemCallError, IOError
        raise Unwritable, @path
      end
    end
  end
end
