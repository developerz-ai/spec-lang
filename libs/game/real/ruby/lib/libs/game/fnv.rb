# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): the hash of a frame's pixels.
# The same in every app with the game shape; change the library, not this file.

module Libs
  module Game
    # FNV-1a, 64 bits: offset 0xcbf29ce484222325, prime 0x100000001b3 = 2^40 + 0x1b3.
    # The state is kept as two 32-bit halves so every product stays a small Integer (no Bignum per byte):
    # h * prime mod 2^64 = h * 0x1b3 + (h << 40), and (h << 40) mod 2^64 only reads the low 24 bits of h.
    module FNV
      module_function

      def hex(bytes)
        hi = 0xcbf29ce4
        lo = 0x84222325
        bytes.each_byte do |b|
          lo ^= b
          low = lo * 0x1b3
          hi = ((hi * 0x1b3) + (low >> 32) + ((lo & 0xffffff) << 8)) & 0xffffffff
          lo = low & 0xffffffff
        end
        format("%<hi>08x%<lo>08x", hi:, lo:)
      end

      # The definition, one Integer of 64 bits; the test oracle for `hex`.
      def reference(bytes)
        h = 0xcbf29ce484222325
        bytes.each_byte { |b| h = ((h ^ b) * 0x100000001b3) & 0xffffffffffffffff }
        format("%<h>016x", h:)
      end
    end
  end
end
