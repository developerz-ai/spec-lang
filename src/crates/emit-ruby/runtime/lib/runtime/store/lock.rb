module Runtime
  module Store
    # flock(2) on the lock file: exclusive for a rule that writes, shared for one that only reads.
    module Lock
      # The lock file cannot be created, opened or locked.
      class Unavailable < StandardError; end

      # Still held by another process when the time was up.
      class Busy < StandardError; end

      TIMEOUT = 5.0
      INTERVAL = 0.01

      # Holds the lock while the block runs. The file is created if missing and never deleted.
      def self.hold(path, exclusive:, timeout: TIMEOUT, interval: INTERVAL)
        file = open_file(path)
        begin
          acquire(file, exclusive ? File::LOCK_EX : File::LOCK_SH, timeout, interval)
          yield
        ensure
          file.close
        end
      end

      def self.open_file(path)
        File.open(path, File::RDONLY | File::CREAT, 0o644)
      rescue SystemCallError => e
        raise Unavailable, e.message
      end

      def self.acquire(file, mode, timeout, interval)
        deadline = now + timeout
        until file.flock(mode | File::LOCK_NB)
          raise Busy if now >= deadline

          sleep(interval)
        end
      rescue SystemCallError => e
        raise Unavailable, e.message
      end

      def self.now = Process.clock_gettime(Process::CLOCK_MONOTONIC)

      private_class_method :open_file, :acquire, :now
    end
  end
end
