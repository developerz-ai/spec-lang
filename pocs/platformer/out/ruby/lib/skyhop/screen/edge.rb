# frozen_string_literal: true

# Projected by hand from the conversions at the foreign edge of hop.screen: Sub, Vec3, Rgb, Box to C (docs/contract.md section 5, src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "raylib"

module Skyhop
  module Screen
    # Spec values to raylib structs. Every `Sub` is at most 2^20, so v / 256 and (lo + hi) / 512 are exact in a
    # C float; a Ruby Float (a double) holds the same value, and libffi's narrowing to float is then exact too.
    module Edge
      module_function

      def vector(x, y, z)
        v = Raylib::Vector3.new
        v[:x] = x
        v[:y] = y
        v[:z] = z
        v
      end

      def vec3(v) = vector(v.x / 256.0, v.y / 256.0, v.z / 256.0)

      def color(rgb)
        c = Raylib::Color.new
        c[:r] = rgb.r
        c[:g] = rgb.g
        c[:b] = rgb.b
        c[:a] = 255
        c
      end

      def bounding_box(box)
        b = Raylib::BoundingBox.new
        b[:min] = vec3(box.lo)
        b[:max] = vec3(box.hi)
        b
      end

      # DrawCube's centre and size.
      def centre(box)
        vector((box.lo.x + box.hi.x) / 512.0, (box.lo.y + box.hi.y) / 512.0, (box.lo.z + box.hi.z) / 512.0)
      end

      def size(box) = [(box.hi.x - box.lo.x) / 256.0, (box.hi.y - box.lo.y) / 256.0, (box.hi.z - box.lo.z) / 256.0]

      def camera(eye, target)
        c = Raylib::Camera3D.new
        c[:position] = vec3(eye)
        c[:target] = vec3(target)
        c[:up] = vector(0.0, 1.0, 0.0)
        c[:fovy] = 45.0
        c[:projection] = Raylib::CAMERA_PERSPECTIVE
        c
      end
    end
  end
end
