# frozen_string_literal: true

# Projected by hand from body_half, crawler_half, gem_half, goal_half, the speeds, Axis, Fall, Placed, body, crawler_box, mover_box, mover_shift, axis, walk_target, facing, slide_x, slide_z, fall, patrol, stomps (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "model"

module Skyhop
  # How bodies move in one tick. Pure: every function takes what it reads.
  module Physics
    Vec3 = Geom::Vec3

    BODY_HALF = Vec3.new(64, 192, 64)
    CRAWLER_HALF = Vec3.new(96, 128, 96)
    GEM_HALF = 64
    GOAL_HALF = Vec3.new(64, 768, 64)

    RUN_SPEED = 16
    DIAGONAL_SPEED = 11
    GROUND_ACCEL = 2
    AIR_ACCEL = 1
    GRAVITY = 2
    MAX_FALL = 48
    JUMP_SPEED = 40
    HOP_CUT = 16
    COYOTE_TICKS = 6
    STOMP_BOUNCE = 24
    STOMP_LINE = 64
    KILL_Y = -2048

    Axis = Data.define(:pos, :vel)
    Fall = Data.define(:y, :vel, :grounded, :riding)
    Placed = Data.define(:no, :box)

    module_function

    def body(at) = Geom.around_feet(at, BODY_HALF)

    def crawler_box(c) = Geom.around_feet(c.at, CRAWLER_HALF)

    def mover_box(m, t) = Geom.shifted(m.home, Geom.scaled(m.step, Geom.triangle(t, m.span)))

    def mover_shift(m, t) = Geom.minus(mover_box(m, t).lo, mover_box(m, t - 1).lo)

    def axis(neg, pos) = (pos ? 1 : 0) - (neg ? 1 : 0)

    def walk_target(pad)
      dx = axis(pad.left, pad.right)
      dz = axis(pad.forward, pad.back)
      speed = dx != 0 && dz != 0 ? DIAGONAL_SPEED : RUN_SPEED
      Vec3.new(dx * speed, 0, dz * speed)
    end

    def facing(pad, was)
      dx = axis(pad.left, pad.right)
      dz = axis(pad.forward, pad.back)
      case dx
      when 1 then { -1 => 45, 1 => 315 }.fetch(dz, 0)
      when -1 then { -1 => 135, 1 => 225 }.fetch(dz, 180)
      else { -1 => 90, 1 => 270 }.fetch(dz, was)
      end
    end

    def slide_x(at, v, solids)
      return Axis.new(at.x, 0) if v.zero?

      moved = body(at.with(x: at.x + v))
      hits = solids.select { |s| Geom.overlaps(moved, s.box) }
      return Axis.new(at.x + v, v) if hits.empty?
      return Axis.new(hits.map { it.box.lo.x }.min - BODY_HALF.x, 0) if v.positive?

      Axis.new(hits.map { it.box.hi.x }.max + BODY_HALF.x, 0)
    end

    def slide_z(at, v, solids)
      return Axis.new(at.z, 0) if v.zero?

      moved = body(at.with(z: at.z + v))
      hits = solids.select { |s| Geom.overlaps(moved, s.box) }
      return Axis.new(at.z + v, v) if hits.empty?
      return Axis.new(hits.map { it.box.lo.z }.min - BODY_HALF.z, 0) if v.positive?

      Axis.new(hits.map { it.box.hi.z }.max + BODY_HALF.z, 0)
    end

    def fall(at, v, solids, tops)
      moved = body(at.with(y: at.y + v))
      hits = solids.select { |s| Geom.overlaps(moved, s.box) }
      return Fall.new(hits.map { it.box.lo.y }.min - BODY_HALF.y, 0, false, nil) if !hits.empty? && v.positive?
      return Fall.new(hits.map { it.box.hi.y }.max, 0, true, nil) unless hits.empty?

      catch_on(at, v, tops)
    end

    # The second half of `fall`: a mover's top only catches feet that were above it.
    def catch_on(at, v, tops)
      here = body(at)
      catching = tops.select do |p|
        v.negative? && Geom.over(here, p.box) && at.y >= p.box.hi.y && at.y + v < p.box.hi.y
      end
      return Fall.new(at.y + v, v, false, nil) if catching.empty?

      top = catching.map { it.box.hi.y }.max
      Fall.new(top, 0, true, catching.select { it.box.hi.y == top }.map(&:no).min)
    end

    def patrol(c)
      x = c.at.x + (c.dir * c.speed)
      return c.with(at: c.at.with(x: c.hi), dir: -1) if x >= c.hi
      return c.with(at: c.at.with(x: c.lo), dir: 1) if x <= c.lo

      c.with(at: c.at.with(x: x))
    end

    def stomps(feet, vy, c) = vy.negative? && feet >= c.at.y + STOMP_LINE
  end
end
