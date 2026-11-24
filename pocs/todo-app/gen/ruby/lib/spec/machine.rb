# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: one world, and the rules that move it.
# The same in every generated project; change the printer, not this file.

module Spec
  # The world of one run: every state's value, each handler's pretend state, and the real handlers and foreign
  # calls a library's runtime serves. Values are never changed in place, so a snapshot is a reference.
  class Machine
    attr_accessor :world
    attr_reader :program

    def initialize(program, world = program.initial_world)
      @program = program
      @world = world
      @handlers = program.initial_handlers
      @served = {}
      @externs = {}
    end

    # A rule call is one atomic step: on any way out but a value, the world and the handlers go back to what
    # they were before the call, for a rule called inside a rule as well.
    def atomic
      world = @world
      handlers = @handlers
      marks = @served.transform_values(&:mark)
      yield
    rescue Exception # rubocop:disable Lint/RescueException -- restores, then raises it again
      @world = world
      @handlers = handlers
      marks&.each { |key, mark| @served.fetch(key).rewind(mark) }
      raise
    end

    # A library's runtime does the real thing for a handler: `perform(op, args)`, `mark`, `rewind(mark)`.
    def serve(key, real) = @served[key] = real

    # A library's runtime implements foreign symbols: `"raylib#DrawCube" => callable`.
    def bind(externs) = @externs.merge!(externs)

    def effect(key, name, args)
      real = @served[key]
      return real.perform(name, args) if real

      handler = @program.handlers.fetch(key)
      op = handler.ops[name]
      Spec.stop!("E-UNSUPPORTED", "`#{key}.#{name}` has no pretend") if op.nil? || op.arm.nil?
      op.params.zip(args) { |param, arg| param.type.check(arg, "argument `#{param.name}` of `#{name}`") if param.type.strict? }
      reply, state = op.arm.call(*@handlers.fetch(handler.index), *args)
      @handlers = @handlers.dup.tap { |all| all[handler.index] = state.freeze }.freeze
      reply
    end

    def extern(symbol, decl, args)
      real = @externs[symbol]
      raise Blocked.new("B-FOREIGN", "reached #{decl}, which is foreign \"#{symbol}\": only a library's runtime runs it") if real.nil?

      real.call(*args)
    end
  end

  class << self
    # The machine every printed rule, state and effect uses. A runner installs one before calling the program.
    attr_accessor :machine

    def world = machine.world

    def world=(world)
      machine.world = world
    end

    def rule(&) = machine.atomic(&)

    def effect(key, name, *args) = machine.effect(key, name, args)

    def extern(symbol, decl, *args) = machine.extern(symbol, decl, args)

    # Runs the block on another machine, then comes back.
    def on(other)
      saved = machine
      self.machine = other
      yield
    ensure
      self.machine = saved
    end
  end

  # A guard: false, or a failure while evaluating it, refuses with the guard's error.
  def self.guard(error)
    held = begin
      yield
    rescue Failure
      false
    end
    fail!(error) unless held
  end
end
