# frozen_string_literal: true

# Projected by hand from Sub, unit, Vec3, Box, plus, minus, scaled, shifted, around_feet, around_centre, overlaps, touches, over, triangle, approach, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  # Points and boxes in sub-metres. Pure.
  module Geom
    UNIT = 256

    Vec3 = Data.define(:x, :y, :z)
    Box = Data.define(:lo, :hi)

    module_function

    def plus(a, b) = Vec3.new(a.x + b.x, a.y + b.y, a.z + b.z)

    def minus(a, b) = Vec3.new(a.x - b.x, a.y - b.y, a.z - b.z)

    def scaled(v, k) = Vec3.new(v.x * k, v.y * k, v.z * k)

    def shifted(b, by) = Box.new(plus(b.lo, by), plus(b.hi, by))

    def around_feet(feet, half)
      Box.new(Vec3.new(feet.x - half.x, feet.y, feet.z - half.z),
              Vec3.new(feet.x + half.x, feet.y + half.y, feet.z + half.z))
    end

    def around_centre(c, half)
      Box.new(Vec3.new(c.x - half, c.y - half, c.z - half), Vec3.new(c.x + half, c.y + half, c.z + half))
    end

    def overlaps(a, b)
      a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.y < b.hi.y && b.lo.y < a.hi.y && a.lo.z < b.hi.z && b.lo.z < a.hi.z
    end

    def touches(a, b)
      a.lo.x <= b.hi.x && b.lo.x <= a.hi.x && a.lo.y <= b.hi.y && b.lo.y <= a.hi.y &&
        a.lo.z <= b.hi.z && b.lo.z <= a.hi.z
    end

    def over(a, b) = a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.z < b.hi.z && b.lo.z < a.hi.z

    # Ruby's Integer#% is floored, as the spec's `mod` is (contract section 1).
    def triangle(t, n)
      p = t % (2 * n)
      p <= n ? p : (2 * n) - p
    end

    def approach(v, target, step) = v < target ? [v + step, target].min : [v - step, target].max

    def vec_text(v) = "#{v.x},#{v.y},#{v.z}"
  end
end
