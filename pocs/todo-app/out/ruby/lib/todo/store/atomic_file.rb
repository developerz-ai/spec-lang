# frozen_string_literal: true

# Projected by hand from the atomic replace after a rule (docs/contract.md section 3, "Store"). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Store
    # Replaces a file so that a reader sees the old bytes or the new, never a part.
    module AtomicFile
      # Write `<path>.<pid>.tmp` beside it in one write, fsync, rename over `<path>`. Raises SystemCallError.
      def self.replace(path, bytes)
        temporary = "#{path}.#{Process.pid}.tmp"
        File.open(temporary, File::WRONLY | File::CREAT | File::TRUNC | File::BINARY, 0o644) do |file|
          file.syswrite(bytes) == bytes.bytesize or raise IOError, "short write to #{temporary}"
          file.fsync
        end
        File.rename(temporary, path)
      rescue SystemCallError, IOError
        discard(temporary)
        raise
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
