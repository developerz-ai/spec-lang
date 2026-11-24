# frozen_string_literal: true

# Projected by hand from rule present and the foreign rules clear_background, begin_mode_3d, end_mode_3d, begin_lit, end_lit, draw_cube, draw_cube_wires, draw_model, draw_text (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module Screen
    # Draws one frame of the world with raylib, in the order the frame lists.
    class Presenter
      WHITE = Edge.color(Model::Rgb.new(255, 255, 255))
      UP = Edge.vector(0.0, 1.0, 0.0)
      ONE = Edge.vector(1.0, 1.0, 1.0)

      def initialize(assets)
        @assets = assets
        @time = FFI::MemoryPointer.new(:float)
      end

      def present(world)
        f = Render.scene(world.view)
        Raylib.ClearBackground(Edge.color(f.sky))
        Raylib.BeginMode3D(Edge.camera(f.eye, f.target))
        begin_lit(f.time)
        f.world.each { draw(it) }
        Raylib.EndShaderMode
        Raylib.EndMode3D
        f.labels.each { |l| Raylib.DrawText(l.text, l.x, l.y, l.size, Edge.color(l.color)) }
      end

      private

      def begin_lit(time)
        @time.put_float(0, time.to_f)
        Raylib.SetShaderValue(@assets.lit, @assets.time_loc, @time, Raylib::SHADER_UNIFORM_FLOAT)
        Raylib.BeginShaderMode(@assets.lit)
      end

      def draw(d)
        case d
        in Render::Cube(box:, color:) then Raylib.DrawCube(Edge.centre(box), *Edge.size(box), Edge.color(color))
        in Render::Wires(box:, color:) then Raylib.DrawCubeWires(Edge.centre(box), *Edge.size(box), Edge.color(color))
        in Render::Mesh(asset:, at:, yaw:)
          Raylib.DrawModelEx(@assets.model(asset), Edge.vec3(at), UP, yaw.to_f, ONE, WHITE)
        end
      end
    end
  end
end
