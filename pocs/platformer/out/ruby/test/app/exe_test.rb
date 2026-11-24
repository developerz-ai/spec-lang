# frozen_string_literal: true

# Projected by hand from the run, draw and shot commands of app SkyhopRuby (src/apps/ruby.slg, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"
require "open3"
require "tmpdir"

module Skyhop
  module TestSupport
    # The real executable in a real process, against the POC's raylib and assets.
    class ExeTest < Minitest::Test
      ENV_VARS = {
        "SKYHOP_RAYLIB" => ENV.fetch("SKYHOP_RAYLIB", File.join(POC, "vendor", "raylib", "lib", "libraylib.so")),
        "SKYHOP_ASSETS" => File.join(POC, "assets")
      }.freeze

      def skyhop(*, input: "", env: {})
        out, err, status = Open3.capture3(ENV_VARS.merge(env), EXE, *, stdin_data: input, binmode: true)
        [out, err, status.exitstatus]
      end

      def script(name) = File.binread(File.join(POC, "parity", "inputs", name))

      def test_run_the_meadow_to_the_flag
        out, err, code = skyhop("run", input: script("meadow-win.txt"))

        assert_equal ["", 0], [err, code]
        assert_equal "0 level meadow\n1 land\n", out[0, 22]
        assert out.end_with?("316 won\nend 316 won gems 3/3 lives 3 at 4992,0,0\n")
      end

      def test_draw_stops_at_the_tick
        out, _err, code = skyhop("draw", "--level", "2", "--at", "300", input: script("ridge-win.txt"))

        assert_equal 0, code
        assert_equal "time 300", out.lines[2].chomp
        assert_equal "mesh pip 2389,292,0 180", out.lines[-3].chomp
      end

      def test_a_bad_script_is_refused_before_any_window
        assert_equal ["", "error: input line 1: unknown key \"Z\"\n", 2], skyhop("shot", input: "1 Z\n")
      end

      def test_shot_hashes_the_same_frame_the_same_way_and_writes_it
        skip "no X server (run the gate under xvfb-run -a)" unless ENV["DISPLAY"]

        Dir.mktmpdir do |dir|
          png = File.join(dir, "frame.png")
          first = skyhop("shot", "--at", "150", "--out", png, input: script("meadow-win.txt"))
          second = skyhop("shot", "--at", "150", input: script("meadow-win.txt"))

          assert_match(/\Ashot [0-9a-f]{16}\n\z/, first[0])
          assert_equal [first[0], "", 0], second
          assert_equal "\x89PNG".b, File.binread(png, 4)
        end
      end

      def test_a_missing_asset_is_exit_3
        skip "no X server (run the gate under xvfb-run -a)" unless ENV["DISPLAY"]

        Dir.mktmpdir do |dir|
          expected = ["", "error: asset #{dir}/out/pip.glb: cannot load\n", 3]

          assert_equal expected, skyhop("shot", env: { "SKYHOP_ASSETS" => dir })
        end
      end

      def test_no_display_is_refused_before_raylib
        assert_equal ["", "error: window: cannot open\n", 3],
                     skyhop("shot", env: { "DISPLAY" => nil, "WAYLAND_DISPLAY" => "" })
      end

      def test_a_missing_library_is_exit_3
        assert_equal ["", "error: raylib /nowhere/libraylib.so: cannot load\n", 3],
                     skyhop("shot", env: { "SKYHOP_RAYLIB" => "/nowhere/libraylib.so", "DISPLAY" => ":0" })
      end
    end
  end
end
