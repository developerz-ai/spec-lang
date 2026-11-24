# frozen_string_literal: true

# Projected by hand from Asset and asset_name (src/hop/render.slg) and the asset loading of docs/contract.md section 4. Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../render"

module Skyhop
  module Screen
    # The lit shader and one model per Asset, loaded after the window opens and unloaded in reverse.
    class Assets
      # An asset that does not load; the message is its path.
      class Unloadable < StandardError; end

      attr_reader :lit, :time_loc

      # SKYHOP_ASSETS wins; otherwise assets/ relative to the working directory.
      def self.root(env) = env.fetch("SKYHOP_ASSETS", "assets")

      def initialize(root)
        @root = root
        @models = {}
        @lit = load_shader
        @time_loc = Raylib.GetShaderLocation(@lit, "uTime")
        Render::ASSETS.each { |asset| @models[asset] = load_model(asset) }
      end

      def model(asset) = @models.fetch(asset)

      def unload
        @models.values.reverse_each { Raylib.UnloadModel(it) }
        @models.clear
        Raylib.UnloadShader(@lit)
      end

      private

      def load_shader
        vs = File.join(@root, "shaders", "lit.vs")
        fs = File.join(@root, "shaders", "lit.fs")
        shader = Raylib.LoadShader(vs, fs)
        raise Unloadable, vs unless Raylib.IsShaderValid(shader)

        shader
      end

      # raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
      def load_model(asset)
        path = File.join(@root, "out", "#{Render.asset_name(asset)}.glb")
        model = Raylib.LoadModel(path)
        raise Unloadable, path unless Raylib.IsModelValid(model)

        model[:materialCount].times do |i|
          material = Raylib::Material.new(model[:materials] + (i * Raylib::Material.size))
          material[:shader] = @lit
        end
        model
      end
    end
  end
end
