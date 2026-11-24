# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): the program's values as C values.
# The same in every app that calls raylib; change the library, not this file.

require_relative "binding"

module Libs
  module Raylib
    # The conversions at the foreign edge. The language has no `f32` and no C struct layout, so a length is a
    # whole number of 1/256 of a raylib unit and becomes `(float)v / 256.0f`; a vector is a record of three
    # lengths; a colour is a record of three or four bytes (alpha 255 when left out); a box is a record of two
    # vectors. Records are read by position, so the program's own names for them do not matter.
    module Edge
      UNIT = 256.0

      module_function

      def vector(x, y, z)
        v = Binding::Vector3.new
        v[:x] = x
        v[:y] = y
        v[:z] = z
        v
      end

      def vec3(record)
        x, y, z = record.deconstruct
        vector(x / UNIT, y / UNIT, z / UNIT)
      end

      def color(record)
        r, g, b, a = record.deconstruct
        c = Binding::Color.new
        c[:r] = r
        c[:g] = g
        c[:b] = b
        c[:a] = a || 255
        c
      end

      def bounding_box(record)
        lo, hi = record.deconstruct
        b = Binding::BoundingBox.new
        b[:min] = vec3(lo)
        b[:max] = vec3(hi)
        b
      end

      # DrawCube's arguments: the centre, then width, height and depth.
      def cube(record)
        lo, hi = record.deconstruct.map(&:deconstruct)
        centre = vector(*lo.zip(hi).map { |a, b| (a + b) / (2 * UNIT) })
        [centre, *lo.zip(hi).map { |a, b| (b - a) / UNIT }]
      end

      # A perspective camera, y up, 45 degrees of vertical view.
      def camera(eye, target)
        c = Binding::Camera3D.new
        c[:position] = vec3(eye)
        c[:target] = vec3(target)
        c[:up] = vector(0.0, 1.0, 0.0)
        c[:fovy] = 45.0
        c[:projection] = Binding::CAMERA_PERSPECTIVE
        c
      end
    end
  end
end
