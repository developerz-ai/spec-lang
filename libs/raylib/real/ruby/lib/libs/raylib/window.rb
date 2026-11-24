# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): one open window and the calls drawn on it.
# The same in every app that calls raylib; change the library, not this file.

require_relative "assets"

module Libs
  module Raylib
    # An open raylib window with its assets loaded. `externs` are the program's foreign calls, by
    # `library#symbol`; a window loop binds them to its machine, then draws with `frame` or `capture`.
    class Window
      WHITE = [255, 255, 255, 255].freeze

      def self.open(program, env:, prefix:, width:, height:, title:, hidden:, fps:)
        Binding.open(Binding.path(env, prefix))
        Binding.SetTraceLogLevel(Binding::LOG_NONE)
        Binding.SetConfigFlags(hidden ? Binding::FLAG_WINDOW_HIDDEN : Binding::FLAG_MSAA_4X_HINT)
        Binding.InitWindow(width, height, title)
        raise Unavailable, "window: cannot open" unless Binding.IsWindowReady

        begin
          assets = Assets.new(Assets.root(env, prefix), Assets.models(program))
          begin
            Binding.SetTargetFPS(fps) if fps
            yield new(assets)
          ensure
            assets.unload
          end
        ensure
          Binding.CloseWindow
        end
      end

      def initialize(assets)
        @assets = assets
        @time = FFI::MemoryPointer.new(:float)
        @up = Edge.vector(0.0, 1.0, 0.0)
        @one = Edge.vector(1.0, 1.0, 1.0)
        @white = Binding::Color.new
        %i[r g b a].zip(WHITE) { |channel, value| @white[channel] = value }
      end

      # Esc or the close button.
      def closing? = Binding.WindowShouldClose

      # One frame on the window.
      def frame
        Binding.BeginDrawing
        yield
      ensure
        Binding.EndDrawing
      end

      # One frame off screen, drawn by `draw`: yields its pixels (R8G8B8A8, top row first) and a way to write
      # them as PNG.
      def capture(width, height, draw)
        target = Binding.LoadRenderTexture(width, height)
        Binding.BeginTextureMode(target)
        draw.call
        Binding.EndTextureMode
        image = Binding.LoadImageFromTexture(target[:texture])
        Binding.ImageFlipVertical(image.pointer) # the texture's first row is the bottom one
        begin
          pixels = image[:data].read_bytes(image[:width] * image[:height] * 4)
          yield pixels, ->(path) { Binding.ExportImage(image, path) }
        ensure
          Binding.UnloadImage(image)
          Binding.UnloadRenderTexture(target)
        end
      end

      # Each `foreign c "raylib#Name"` the contract lists, taking the program's values.
      def externs
        {
          "raylib#ClearBackground" => ->(color) { Binding.ClearBackground(Edge.color(color)) },
          "raylib#BeginMode3D" => ->(eye, target) { Binding.BeginMode3D(Edge.camera(eye, target)) },
          "raylib#EndMode3D" => -> { Binding.EndMode3D },
          "raylib#BeginShaderMode" => ->(time) { begin_shader(time) },
          "raylib#EndShaderMode" => -> { Binding.EndShaderMode },
          "raylib#DrawCube" => ->(box, color) { Binding.DrawCube(*Edge.cube(box), Edge.color(color)) },
          "raylib#DrawCubeWires" => ->(box, color) { Binding.DrawCubeWires(*Edge.cube(box), Edge.color(color)) },
          "raylib#DrawModelEx" => ->(model, at, yaw) { draw_model(model, at, yaw) },
          "raylib#DrawText" => ->(text, x, y, size, color) { Binding.DrawText(text, x, y, size, Edge.color(color)) },
          "raylib#IsKeyDown" => ->(key) { Binding.IsKeyDown(key) },
          "raylib#CheckCollisionBoxes" => lambda do |a, b|
            Binding.CheckCollisionBoxes(Edge.bounding_box(a), Edge.bounding_box(b))
          end
        }.transform_values { |call| unit(call) }
      end

      private

      # A C function that returns nothing gives `()`.
      def unit(call)
        lambda do |*args|
          result = call.call(*args)
          [true, false].include?(result) ? result : nil
        end
      end

      # The shader's one uniform is the time; then everything until EndShaderMode is drawn through it.
      def begin_shader(time)
        @time.put_float(0, time.to_f)
        Binding.SetShaderValue(@assets.lit, @assets.time_loc, @time, Binding::SHADER_UNIFORM_FLOAT)
        Binding.BeginShaderMode(@assets.lit)
      end

      def draw_model(model, at, yaw)
        Binding.DrawModelEx(@assets.model(model), Edge.vec3(at), @up, yaw.to_f, @one, @white)
      end
    end
  end
end
