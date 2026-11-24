# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: the program as data.
# The same in every generated project; change the printer, not this file.

module Spec
  # The argument a caller leaves out: the declaration's own default applies.
  ABSENT = Object.new.freeze

  # Keyword arguments without the absent ones.
  def self.given(**args) = args.reject { |_, value| value.equal?(ABSENT) }

  # A parameter of a rule, a fn, a law, a test or a handler operation. `type` is its declared type; an optional
  # one has a default in the declaration.
  class Param
    attr_reader :name

    def initialize(name, type, optional: false)
      @name = name
      @type = type
      @optional = optional
    end

    def type = @type.call

    def optional? = @optional
  end

  # A rule or a fn as a library's runtime and the trace generator see it. `call` takes every argument, in order.
  class Callable
    attr_reader :id, :name, :params, :triggers, :why, :uses, :extern

    def initialize(id, name, params, body, result: nil, triggers: [], why: nil, uses: [], extern: nil)
      @id = id
      @name = name
      @params = params
      @body = body
      @result = result
      @triggers = triggers
      @why = why
      @uses = uses
      @extern = extern
    end

    def call(args) = @body.call(args)

    # The declared result type, or nil for `()`.
    def result = @result&.call

    # `board.add`: the last part of its module, then its name, as a trace shows a call.
    def short = @short ||= name.split(".").last(2).join(".")

    def trigger(name) = triggers.find { |note| note.name == name }
  end

  class Rule < Callable; end
  class Fn < Callable; end

  # A law: a Bool over a trace and its other parameters.
  Law = Data.define(:name, :params, :trace, :traced, :start, :body)

  # One state: its place in the world, declared type and initial value.
  class State
    attr_reader :name, :member

    def initialize(name, member, type, init)
      @name = name
      @member = member
      @type = type
      @init = init
    end

    def type = @type.call

    def initial = @init.call
  end

  # A library's handler: a group of operations (`out.line`), and what tests run for each (its pretend).
  Handler = Data.define(:key, :index, :state, :ops)

  # One operation: who may use it (:rule, :fn or :test), its parameters, its pretend arm (nil: none).
  Operation = Data.define(:kind, :params, :arm)

  # A library's note or trigger with its arguments by parameter name: `bin("todo")`, `tick(60Hz)`.
  Note = Data.define(:library, :name, :args) do
    def [](param) = args.fetch(param)
  end

  # `60Hz` and `file(".todo.json")` as note arguments.
  Quantity = Data.define(:value, :unit)
  Choice = Data.define(:name, :args)

  # One `app`: what a library's runtime reads.
  App = Data.define(:name, :qualified, :target, :shape, :notes, :params, :rules) do
    def note(name) = notes.find { |note| note.name == name }
  end

  # Every declaration a runtime may need by name. Built once by the printed `lib/program.rb`.
  class Program
    attr_reader :states, :rules, :fns, :laws, :handlers, :apps

    def initialize
      @states = []
      @rules = {}
      @fns = {}
      @laws = []
      @handlers = {}
      @apps = {}
      yield self
    end

    def state(name, member, type, init) = @states << State.new(name, member, type, init)

    def rule(name, params, body, **) = @rules[name] = Rule.new(@rules.size, name, params, body, **)

    def fn(name, params, body, **) = @fns[name] = Fn.new(@fns.size, name, params, body, **)

    def law(name, params, body, trace:, traced:, start:)
      @laws << Law.new(name:, params:, trace:, traced:, start:, body:)
    end

    # `state` is [name, initial value] per pretend state; `ops` is name => Operation.
    def handler(key, state: [], ops: {})
      @handlers[key] = Handler.new(key:, index: @handlers.size, state:, ops:)
    end

    def app(name, **) = @apps[name] = App.new(name:, **)

    def param(name, type, **) = Param.new(name, type, **)

    def note(library, name, **args) = Note.new(library:, name:, args: args.transform_keys(&:to_s))

    def operation(kind, params = [], &arm) = Operation.new(kind:, params:, arm:)

    # The world's class: one member per state, in declaration order.
    def world_class
      @world_class ||= begin
        names = states.map(&:name)
        Data.define(*states.map(&:member)) { define_singleton_method(:state_names) { names } }
      end
    end

    # Every state at its initial value, each checked against its type. Remembered: values never change in place.
    def initial_world
      @initial_world ||= begin
        world = world_class.new(*states.map(&:initial))
        states.each { |s| s.type.check(world.public_send(s.member), "the initial value of #{s.name}") if s.type.strict? }
        world
      end
    end

    def initial_handlers = @initial_handlers ||= handlers.values.map { |h| h.state.map { |_, init| init.call }.freeze }.freeze
  end
end
