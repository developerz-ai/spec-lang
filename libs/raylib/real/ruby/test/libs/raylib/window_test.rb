# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): tests through the real library, on the app
# it was built into.
# The same in every app that calls raylib; change the library, not this file.

require "minitest/autorun"
require "app"
require "libs/raylib"

module Libs
  module Raylib
    class WindowTest < Minitest::Test
      # The app's executable name in capitals: the prefix of the variables that say where raylib and the assets are.
      PREFIX = App::MANIFEST.note("bin")["name"].upcase

      # The environment with the assets found: where the variable says, else the nearest `assets/` above the project.
      def environment
        return ENV.to_h unless ENV.fetch("#{PREFIX}_ASSETS", "").empty?

        folder = File.expand_path("../../..", __dir__)
        folder = File.dirname(folder) until File.file?(File.join(folder, "assets/shaders/lit.vs")) || folder == "/"
        ENV.to_h.merge("#{PREFIX}_ASSETS" => File.join(folder, "assets"))
      end

      def open_window(&)
        env = environment
        skip "no display: start the tests under xvfb-run" if env.fetch("DISPLAY", "").empty?
        skip "no assets: set #{PREFIX}_ASSETS" unless File.file?(File.join(env["#{PREFIX}_ASSETS"], "shaders/lit.vs"))

        require "libs/raylib/assets"
        Raylib.open(PROGRAM, env:, prefix: PREFIX, width: 64, height: 36, title: "test", hidden: true, &)
      end

      def test_without_a_display_no_window_opens
        error = assert_raises(Unavailable) do
          Raylib.open(PROGRAM, env: {}, prefix: PREFIX, width: 1, height: 1, title: "t") { flunk }
        end
        assert_equal "window: cannot open", error.message
      end

      def test_a_length_is_a_256th_of_a_unit
        require "libs/raylib/edge"
        point = Data.define(:x, :y, :z)
        box = Data.define(:lo, :hi).new(point.new(-256, 0, 128), point.new(256, 512, 384))
        centre, width, height, depth = Edge.cube(box)
        assert_equal [0.0, 1.0, 1.0], [centre[:x], centre[:y], centre[:z]]
        assert_equal [2.0, 2.0, 1.0], [width, height, depth]
        assert_equal [1, 2, 3, 255], Edge.color(point.new(1, 2, 3)).then { |c| %i[r g b a].map { c[it] } }
      end

      def test_every_foreign_declaration_of_the_program_is_served
        open_window do |window|
          declared = (PROGRAM.rules.values + PROGRAM.fns.values).filter_map(&:extern).select { it.start_with?("raylib#") }
          assert_empty declared - window.externs.keys
        end
      end

      def test_a_frame_off_screen_has_four_bytes_a_pixel
        open_window do |window|
          black = Data.define(:r, :g, :b).new(0, 0, 0)
          window.capture(64, 36, -> { window.externs.fetch("raylib#ClearBackground").call(black) }) do |pixels, _export|
            assert_equal 64 * 36 * 4, pixels.bytesize
            assert_equal [0, 0, 0, 255], pixels.bytes.first(4)
          end
        end
      end

      def test_a_missing_asset_is_named
        missing = ENV.to_h.merge("#{PREFIX}_ASSETS" => "/no/such/assets")
        skip "no display: start the tests under xvfb-run" if ENV.fetch("DISPLAY", "").empty?
        error = assert_raises(Unavailable) do
          Raylib.open(PROGRAM, env: missing, prefix: PREFIX, width: 8, height: 8, title: "t", hidden: true) { flunk }
        end
        assert_match(%r{\Aasset /no/such/assets/.+: cannot load\z}, error.message)
      end
    end
  end
end
