# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: the engine's value generator.
# The same in every generated project; change the printer, not this file.

module Spec
  module Testing
    # Values a generator may reuse: the keys of each state declared `Map[K, V]` and the elements of each declared
    # `Set[K]`, by exactly that type.
    class Pools
      EMPTY = new.freeze

      def self.from_worlds(program, worlds)
        pools = new
        worlds.each do |world|
          program.states.each do |state|
            type = state.type
            value = world.public_send(state.member)
            case type.kind
            when :map then pools.add(type.key, value.keys)
            when :set then pools.add(type.of, value.to_a)
            end
          end
        end
        pools
      end

      def initialize
        @pools = {}.compare_by_identity
        @sorted = {}.compare_by_identity
      end

      def add(type, values)
        (@pools[type] ||= []).concat(values)
        @sorted.delete(type)
      end

      # The values of exactly this type, ascending, or nil when there are none.
      def get(type)
        return nil unless @pools&.key?(type)

        values = @sorted[type] ||= Spec.sort(@pools[type].uniq)
        values.empty? ? nil : values
      end
    end

    # Draws values of a declared type exactly as the engine does, so a seed reproduces its run here.
    class Generator
      TRIES = 400
      EDGES = [
        0, 1, -1, 2_147_483_647, 2_147_483_648, 9_007_199_254_740_992, 9_007_199_254_740_993,
        9_223_372_036_854_775_807, 9_223_372_036_854_775_808, -9_223_372_036_854_775_808, 18_446_744_073_709_551_616
      ].freeze
      WORD = "abcdefghijklmnopqrstuvwxyz0123456789-".chars.freeze
      MIX = [
        "a", "Z", " ", "\t", "\n", "é", "ß", "中", "😀", "　", "\u0085", " ", "​", "﻿",
        "\u007F", "\u0001", '"', "\\", "{", "}", "#", "-"
      ].freeze

      def initialize(rng) = @rng = rng

      # A value of the type: half the time one from its pool when it has one, else a fresh one; drawn again
      # until it conforms.
      def value(type, pools = Pools::EMPTY)
        TRIES.times do
          pool = pools.get(type)
          candidate = pool && @rng.chance(1, 2) ? @rng.pick(pool) : base(type, pools)
          return candidate if type.conforms?(candidate)
        end
        Spec.stop!("E-GEN", "no value of the type found in #{TRIES} tries")
      end

      private

      def base(type, pools)
        case type.kind
        when :unit then nil
        when :bool then @rng.chance(1, 2)
        when :int then int
        when :text then text
        when :range then in_range(type)
        when :option then @rng.chance(1, 3) ? NONE : Some.new(value(type.of, pools))
        when :list then Array.new(@rng.below(4)) { value(type.of, pools) }
        when :set then Sets.of(Array.new(@rng.below(4)) { value(type.of, pools) })
        when :map then Maps.of(Array.new(@rng.below(4)) { [value(type.key, pools), value(type.of, pools)] })
        when :tuple then Tuple.new(type.items.map { |item| value(item, pools) })
        when :where then value(type.base, pools)
        when :alias then value(type.of, pools)
        when :record then type.new(*type.field_types.map { |field| value(field, pools) })
        when :sum then variant(type, pools)
        else Spec.stop!("E-GEN", "no value of a #{type.kind} can be drawn")
        end
      end

      def variant(type, pools)
        chosen = type.variants.fetch(@rng.below(type.variants.size))
        return chosen unless chosen.is_a?(Class)

        chosen.new(*chosen.field_types.map { |field| value(field, pools) })
      end

      def in_range(type)
        return @rng.chance(1, 2) ? type.lo : type.last if @rng.chance(1, 5)

        @rng.between(type.lo, type.last)
      end

      def int
        case @rng.below(100)
        when 0...35 then @rng.between(0, 10)
        when 35...55 then @rng.between(11, 100)
        when 55...70 then @rng.between(-10, -1)
        when 70...80 then @rng.pick(EDGES)
        when 80...90 then @rng.between(101, 1 << 62)
        else -@rng.between(11, 1 << 62)
        end
      end

      def text
        case @rng.below(4)
        when 0 then Array.new(1 + @rng.below(8)) { @rng.pick(WORD) }.join
        when 1 then Array.new(@rng.below(13)) { (32 + @rng.below(95)).chr(Encoding::UTF_8) }.join
        when 2 then Array.new(@rng.below(9)) { @rng.pick(MIX) }.join
        else ""
        end
      end
    end
  end
end
