module Runtime
  module Store
    # Replaces a file so that a reader sees the old bytes or the new, never a part.
    module AtomicFile
      # Write `<path>.<pid>.tmp` beside it in one write, fsync, rename over `<path>`. Raises SystemCallError.
      # On every way out, failure, bug or signal, the store is the old bytes or the new, whole, and the
      # tmp file is gone: an interrupt is deferred until the rename is done, and `ensure` removes the
      # tmp file unless it was renamed.
      def self.replace(path, bytes)
        temporary = "#{path}.#{Process.pid}.tmp"
        renamed = false
        Thread.handle_interrupt(Object => :never) do
          File.open(temporary, File::WRONLY | File::CREAT | File::TRUNC | File::BINARY, 0o644) do |file|
            file.syswrite(bytes) == bytes.bytesize or raise IOError, "short write to #{temporary}"
            file.fsync
          end
          File.rename(temporary, path)
          renamed = true
        ensure
          discard(temporary) unless renamed
        end
      end

      # Best effort: the failure that brought us here is the one to report.
      def self.discard(path)
        File.unlink(path)
      rescue SystemCallError
        nil
      end

      private_class_method :discard
    end
  end
end
