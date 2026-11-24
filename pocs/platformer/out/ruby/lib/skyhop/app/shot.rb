# frozen_string_literal: true

# Projected by hand from the shot command the game vocabulary derives from tick and frame (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "window"
require_relative "fnv"

module Skyhop
  module App
    # Draws one frame off screen, hashes its pixels, and writes it as PNG when asked.
    module Shot
      WIDTH = 320
      HEIGHT = 180

      module_function

      def call(command, world, out:, err:, env:)
        Window.open(env:, err:, flags: Screen::Raylib::FLAG_WINDOW_HIDDEN, width: WIDTH, height: HEIGHT) do |assets|
          pixels(world, assets) { |image| finish(image, command.out, out:, err:) }
        end
      end

      def pixels(world, assets)
        rt = Screen::Raylib.LoadRenderTexture(WIDTH, HEIGHT)
        Screen::Raylib.BeginTextureMode(rt)
        Screen::Presenter.new(assets).present(world)
        Screen::Raylib.EndTextureMode
        image = Screen::Raylib.LoadImageFromTexture(rt[:texture])
        Screen::Raylib.ImageFlipVertical(image.pointer)
        begin
          yield image
        ensure
          Screen::Raylib.UnloadImage(image)
          Screen::Raylib.UnloadRenderTexture(rt)
        end
      end

      def finish(image, path, out:, err:)
        bytes = image[:data].read_bytes(image[:width] * image[:height] * 4)
        return Window.refuse(err, "--out #{path}: cannot write") if path && !Screen::Raylib.ExportImage(image, path)

        out.write("shot #{FNV.hex(bytes)}\n")
        CLI::OK
      end
    end
  end
end
