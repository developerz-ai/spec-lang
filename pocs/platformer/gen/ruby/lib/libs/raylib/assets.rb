# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): the shader and the models a window loads.
# The same in every app that calls raylib; change the library, not this file.

require_relative "edge"

module Libs
  module Raylib
    # The lit shader (`shaders/lit.vs`, `shaders/lit.fs`, uniform `uTime`) and one model per variant of the sum the
    # program's `raylib#DrawModelEx` declaration takes first, in declared order: `out/<name>.glb`, where the name
    # is what the program's one `fn` from that sum to Text gives, or the variant's name in lower case when there
    # is no such `fn`. Loaded after the window opens, unloaded in reverse.
    class Assets
      attr_reader :lit, :time_loc

      # `<PREFIX>_ASSETS` wins; otherwise `assets/` relative to the working directory.
      def self.root(env, prefix)
        named = env.fetch("#{prefix}_ASSETS", "")
        named.empty? ? "assets" : named
      end

      # The models' sum and the fn that names their files, from the program's declarations.
      def self.models(program)
        draw = (program.rules.values + program.fns.values).find { |d| d.extern == "raylib#DrawModelEx" }
        sum = draw&.params&.first&.type
        return [] unless sum.respond_to?(:variants)

        namer = program.fns.values.select do |f|
          f.params.size == 1 && f.params.first.type.equal?(sum) && f.result.equal?(Spec::Type::TEXT) && f.extern.nil?
        end
        sum.variants.map { |v| [v, namer.size == 1 ? namer.first.call([v]) : v.variant_name.downcase] }
      end

      def initialize(root, models)
        @root = root
        @models = {}.compare_by_identity
        @lit = load_shader
        @time_loc = Binding.GetShaderLocation(@lit, "uTime")
        models.each { |variant, name| @models[variant] = load_model(name) }
      rescue Unavailable
        unload
        raise
      end

      # A model by the variant that names it, or by its index in load order.
      def model(key) = key.is_a?(Integer) ? @models.values.fetch(key) : @models.fetch(key)

      def unload
        @models.values.reverse_each { Binding.UnloadModel(it) }
        @models.clear
        Binding.UnloadShader(@lit) if @lit
        @lit = nil
      end

      private

      def load_shader
        vs = File.join(@root, "shaders", "lit.vs")
        fs = File.join(@root, "shaders", "lit.fs")
        shader = Binding.LoadShader(vs, fs)
        raise Unavailable, "asset #{vs}: cannot load" unless Binding.IsShaderValid(shader)

        shader
      end

      # raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
      def load_model(name)
        path = File.join(@root, "out", "#{name}.glb")
        model = Binding.LoadModel(path)
        raise Unavailable, "asset #{path}: cannot load" unless Binding.IsModelValid(model)

        model[:materialCount].times do |i|
          material = Binding::Material.new(model[:materials] + (i * Binding::Material.size))
          material[:shader] = @lit
        end
        model
      end
    end
  end
end
