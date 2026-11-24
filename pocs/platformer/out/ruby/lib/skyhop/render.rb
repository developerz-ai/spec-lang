# frozen_string_literal: true

# Projected by hand from Asset, Draw, Label, Frame, sky, edge, ink, eye_offset, look_height, asset_name, bob, block, banner, scene, rgb_text, draw_text, frame_lines, dump (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"

module Skyhop
  # What one frame shows, as data. Pure.
  module Render
    Rgb = Model::Rgb
    Vec3 = Geom::Vec3

    # Asset is the symbol of its variant, in declared order.
    ASSETS = %i[PipMesh CrawlerMesh GemMesh FlagMesh].freeze
    ASSET_NAMES = { PipMesh: "pip", CrawlerMesh: "crawler", GemMesh: "gem", FlagMesh: "flag" }.freeze

    Cube = Data.define(:box, :color)
    Wires = Data.define(:box, :color)
    Mesh = Data.define(:asset, :at, :yaw)
    Label = Data.define(:x, :y, :size, :text, :color)
    Frame = Data.define(:sky, :eye, :target, :time, :world, :labels)

    SKY = Rgb.new(135, 206, 235)
    EDGE = Rgb.new(30, 40, 30)
    INK = Rgb.new(255, 255, 255)
    EYE_OFFSET = Vec3.new(0, 512, 2048)
    LOOK_HEIGHT = 192

    BANNERS = {
      Playing: [].freeze,
      Won: [Label.new(380, 250, 40, "level clear", INK)].freeze,
      Lost: [Label.new(400, 250, 40, "game over", INK)].freeze
    }.freeze

    module_function

    def asset_name(asset) = ASSET_NAMES.fetch(asset)

    def bob(t) = (Geom.triangle(t, 32) * 2) - 32

    def block(box, color) = [Cube.new(box, color), Wires.new(box, EDGE)]

    def banner(mode) = BANNERS.fetch(mode)

    def scene(v)
      target = Geom.plus(v.player.at, Vec3.new(0, LOOK_HEIGHT, 0))
      Frame.new(SKY, Geom.plus(target, EYE_OFFSET), target, v.ticks, draws(v), hud(v) + banner(v.mode))
    end

    def draws(v)
      t = v.ticks
      list = v.stage.solids.flat_map { block(it.box, it.color) }
      v.stage.movers.each { |m| list.concat(block(Physics.mover_box(m, t), m.color)) }
      v.stage.gems.each do |g|
        list << Mesh.new(:GemMesh, g.at.with(y: g.at.y + bob(t)), (t * 3) % 360) unless v.taken.include?(g.no)
      end
      v.crawlers.each { |c| list << Mesh.new(:CrawlerMesh, c.at, c.dir.negative? ? 180 : 0) }
      list << Mesh.new(:FlagMesh, v.stage.goal, 0)
      list << Mesh.new(:PipMesh, v.player.at, v.player.yaw)
    end

    def hud(v)
      [Label.new(16, 12, 20, "gems #{v.taken.size}/#{v.stage.gems.size}", INK),
       Label.new(16, 38, 20, "lives #{v.lives}", INK)]
    end

    def rgb_text(c) = "#{c.r},#{c.g},#{c.b}"

    def draw_text(d)
      case d
      in Cube(box:, color:) then "cube #{Geom.vec_text(box.lo)} #{Geom.vec_text(box.hi)} #{rgb_text(color)}"
      in Wires(box:, color:) then "wires #{Geom.vec_text(box.lo)} #{Geom.vec_text(box.hi)} #{rgb_text(color)}"
      in Mesh(asset:, at:, yaw:) then "mesh #{asset_name(asset)} #{Geom.vec_text(at)} #{yaw}"
      end
    end

    def frame_lines(f)
      ["sky #{rgb_text(f.sky)}", "camera #{Geom.vec_text(f.eye)} #{Geom.vec_text(f.target)}", "time #{f.time}"] +
        f.world.map { draw_text(it) } +
        f.labels.map { |l| "label #{l.x} #{l.y} #{l.size} #{rgb_text(l.color)} #{l.text}" }
    end

    def dump(world) = frame_lines(scene(world.view))
  end
end
