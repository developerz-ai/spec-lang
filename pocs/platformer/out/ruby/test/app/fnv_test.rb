# frozen_string_literal: true

# Projected by hand from the pixel hash of the shot command (docs/contract.md section 2, shot). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"
require_relative "../../lib/skyhop/app/fnv"

module Skyhop
  module TestSupport
    # The split-halves FNV-1a against the definition, and the published test vectors.
    class FNVTest < Minitest::Test
      FNV = App::FNV

      def test_published_vectors
        assert_equal "cbf29ce484222325", FNV.hex("")
        assert_equal "af63dc4c8601ec8c", FNV.hex("a")
        assert_equal "85944171f73967e8", FNV.hex("foobar")
      end

      def test_matches_the_definition
        rng = Random.new(20_261_124)
        200.times do
          bytes = rng.bytes(rng.rand(0..600))

          assert_equal FNV.reference(bytes), FNV.hex(bytes)
        end
      end
    end
  end
end
