# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): the binding, through the ffi gem.
# The same in every app that calls raylib; change the library, not this file.

begin
  require "ffi"
rescue LoadError
  # Started with RubyGems off: the load path `bundle install --standalone` wrote (bin/build).
  require_relative "../../../vendor/bundle/bundler/setup"
  require "ffi"
end

module Libs
  module Raylib
    # raylib 5.5's functions and the structs they take. Struct layouts are raylib.h's; every struct the API
    # takes or returns by value is declared `by_value`.
    module Binding
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

      # Where the shared library is: `<PREFIX>_RAYLIB`, else `vendor/raylib/lib/libraylib.so` in the nearest
      # folder above this project that has one, else whatever the system calls `raylib`.
      def self.path(env, prefix)
        named = env.fetch("#{prefix}_RAYLIB", "")
        return named unless named.empty?

        folder = File.expand_path("../../..", __dir__)
        until File.dirname(folder) == folder
          found = File.join(folder, "vendor/raylib/lib/libraylib.so")
          return found if File.file?(found)

          folder = File.dirname(folder)
        end
        "raylib"
      end

      # Opens the library once per process and attaches every function.
      def self.open(path)
        return if @opened

        ffi_lib path
        FUNCTIONS.each { |name, (args, ret)| attach_function name, args, ret }
        @opened = true
      rescue LoadError, FFI::NotFoundError
        raise Unavailable, "raylib #{path}: cannot load"
      end
    end
  end
end
