# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: the engine's random numbers.
# The same in every generated project; change the printer, not this file.

module Spec
  module Testing
    # SplitMix64, as `spec-lang test` uses it: the same seed draws the same numbers.
    class Rng
      MASK = 0xFFFF_FFFF_FFFF_FFFF
      GOLDEN = 0x9E37_79B9_7F4A_7C15

      def initialize(seed) = @state = seed & MASK

      def next_u64
        @state = (@state + GOLDEN) & MASK
        z = @state
        z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & MASK
        z ^ (z >> 31)
      end

      def below(count) = count.zero? ? 0 : next_u64 % count

      # Uniform in lo..=hi, from 128 random bits.
      def between(lo, hi)
        return lo if hi <= lo

        high = next_u64
        lo + (((high << 64) | next_u64) % (hi - lo + 1))
      end

      def chance(num, den) = below(den) < num

      def pick(items) = items[below(items.size)]

      # The seed of run `index` of the check `name`: FNV-1a of the name, mixed with the base seed and the run.
      def self.seed_for(base, name, index)
        hash = 0xcbf2_9ce4_8422_2325
        name.each_byte { |byte| hash = ((hash ^ byte) * 0x0100_0000_01b3) & MASK }
        rotated = ((base << 17) | (base >> 47)) & MASK
        new(hash ^ rotated ^ ((index * GOLDEN) & MASK)).next_u64
      end
    end
  end
end
