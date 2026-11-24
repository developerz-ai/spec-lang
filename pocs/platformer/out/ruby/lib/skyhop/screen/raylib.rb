# frozen_string_literal: true

# Projected by hand from every foreign c "raylib#..." declaration of hop.screen, and the calls contract sections 2 and 4 add (src/hop/screen.slg, docs/contract.md). Stand-in for spec-lang build output; change the spec, not this file.

begin
  require "ffi"
rescue LoadError
  # Started with RubyGems off (exe/skyhop): the load path bin/build wrote with `bundle install --standalone`.
  require_relative "../../../vendor/bundle/bundler/setup"
  require "ffi"
end

module Skyhop
  module Screen
    # The hand-written binding to raylib 5.5, through the ffi gem (libffi). Struct layouts are raylib.h's.
    # Every struct the API takes or returns by value is declared with `by_value`; no C shim is needed.
    module Raylib
      extend FFI::Library

      class Vector3 < FFI::Struct
        layout :x, :float, :y, :float, :z, :float
      end

      class Color < FFI::Struct
        layout :r, :uchar, :g, :uchar, :b, :uchar, :a, :uchar
      end

      class BoundingBox < FFI::Struct
        layout :min, Vector3, :max, Vector3
      end

      class Camera3D < FFI::Struct
        layout :position, Vector3, :target, Vector3, :up, Vector3, :fovy, :float, :projection, :int
      end

      class Shader < FFI::Struct
        layout :id, :uint, :locs, :pointer
      end

      class Matrix < FFI::Struct
        layout(*(0..15).flat_map { [:"m#{it}", :float] })
      end

      class Material < FFI::Struct
        layout :shader, Shader, :maps, :pointer, :params, [:float, 4]
      end

      class Model < FFI::Struct
        layout :transform, Matrix, :meshCount, :int, :materialCount, :int, :meshes, :pointer,
               :materials, :pointer, :meshMaterial, :pointer, :boneCount, :int, :bones, :pointer, :bindPose, :pointer
      end

      class Texture < FFI::Struct
        layout :id, :uint, :width, :int, :height, :int, :mipmaps, :int, :format, :int
      end

      class RenderTexture < FFI::Struct
        layout :id, :uint, :texture, Texture, :depth, Texture
      end

      class Image < FFI::Struct
        layout :data, :pointer, :width, :int, :height, :int, :mipmaps, :int, :format, :int
      end

      LOG_NONE = 7
      FLAG_MSAA_4X_HINT = 0x20
      FLAG_WINDOW_HIDDEN = 0x80
      SHADER_UNIFORM_FLOAT = 0
      CAMERA_PERSPECTIVE = 0

      FUNCTIONS = {
        SetTraceLogLevel: [[:int], :void],
        SetConfigFlags: [[:uint], :void],
        InitWindow: [%i[int int string], :void],
        IsWindowReady: [[], :bool],
        WindowShouldClose: [[], :bool],
        CloseWindow: [[], :void],
        SetTargetFPS: [[:int], :void],
        BeginDrawing: [[], :void],
        EndDrawing: [[], :void],
        ClearBackground: [[Color.by_value], :void],
        BeginMode3D: [[Camera3D.by_value], :void],
        EndMode3D: [[], :void],
        LoadShader: [%i[string string], Shader.by_value],
        IsShaderValid: [[Shader.by_value], :bool],
        GetShaderLocation: [[Shader.by_value, :string], :int],
        SetShaderValue: [[Shader.by_value, :int, :pointer, :int], :void],
        BeginShaderMode: [[Shader.by_value], :void],
        EndShaderMode: [[], :void],
        UnloadShader: [[Shader.by_value], :void],
        LoadModel: [[:string], Model.by_value],
        IsModelValid: [[Model.by_value], :bool],
        UnloadModel: [[Model.by_value], :void],
        DrawModelEx: [[Model.by_value, Vector3.by_value, Vector3.by_value, :float, Vector3.by_value, Color.by_value],
                      :void],
        DrawCube: [[Vector3.by_value, :float, :float, :float, Color.by_value], :void],
        DrawCubeWires: [[Vector3.by_value, :float, :float, :float, Color.by_value], :void],
        DrawText: [[:string, :int, :int, :int, Color.by_value], :void],
        IsKeyDown: [[:int], :bool],
        CheckCollisionBoxes: [[BoundingBox.by_value, BoundingBox.by_value], :bool],
        LoadRenderTexture: [%i[int int], RenderTexture.by_value],
        UnloadRenderTexture: [[RenderTexture.by_value], :void],
        BeginTextureMode: [[RenderTexture.by_value], :void],
        EndTextureMode: [[], :void],
        LoadImageFromTexture: [[Texture.by_value], Image.by_value],
        ImageFlipVertical: [[:pointer], :void],
        ExportImage: [[Image.by_value, :string], :bool],
        UnloadImage: [[Image.by_value], :void]
      }.freeze

      # The library raylib is loaded from: SKYHOP_RAYLIB, else the POC's own vendor/raylib build.
      DEFAULT_PATH = File.expand_path("../../../../../vendor/raylib/lib/libraylib.so", __dir__)

      # The library could not be opened, or lacks a function.
      class Missing < StandardError; end

      def self.path(env) = env.fetch("SKYHOP_RAYLIB", DEFAULT_PATH)

      # Opens the library once per process and attaches every function. Raises Missing.
      def self.open(path)
        return if @opened

        ffi_lib path
        FUNCTIONS.each { |name, (args, ret)| attach_function name, args, ret }
        @opened = true
      rescue LoadError, FFI::NotFoundError => e
        raise Missing, e.message
      end
    end
  end
end
