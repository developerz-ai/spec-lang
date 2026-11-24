# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: the ways an evaluation ends without a value.
# The same in every generated project; change the printer, not this file.

module Spec
  # A language failure: `fail Name(data)`, `Missing`, `DivByZero`, `Uneven`. Handled by `else`, by a rule's
  # refusal and by a test's `-> fail Name`. As a value it is what a refused trace step replies.
  class Failure < StandardError
    attr_reader :name, :data

    def initialize(name, data = [])
      @name = name
      @data = data
      super(name)
    end

    def ==(other) = other.is_a?(Failure) && name == other.name && data == other.data
    alias eql? ==

    def hash = [name, data].hash
  end

  # Evaluation stopped and nothing handles it: a broken claim (E-CLAIM), an operation outside its domain
  # (E-TYPE-MISMATCH, E-TYPE-ARITY, E-RUNTIME), an open choice (E-OPEN). `code` is the toolchain's.
  class Stop < StandardError
    attr_reader :code

    def initialize(code, message)
      @code = code
      super("#{code}: #{message}")
    end
  end

  # A `?hole` (F-BLOCKED) or a foreign body nothing here runs (B-FOREIGN): the check is blocked, not failed.
  class Blocked < StandardError
    attr_reader :code

    def initialize(code, message)
      @code = code
      super("#{code}: #{message}")
    end
  end

  # A test step or an assert that did not hold (F-TEST).
  class CheckFailed < StandardError; end

  def self.fail!(name, *data) = raise(Failure.new(name, data))

  def self.missing! = raise(Failure.new("Missing"))

  def self.stop!(code, message) = raise(Stop.new(code, message))

  def self.hole!(name) = raise(Blocked.new("F-BLOCKED", "reached ?#{name}"))

  # `assert cond;`: false ends the check that reached it.
  def self.assert(held)
    stop!("E-TYPE-MISMATCH", "a condition is a Bool") unless [true, false].include?(held)
    raise CheckFailed, "an assert is false" unless held
  end

  # `value else fallback` on an Option: the Option, or `none` when evaluating it fails.
  def self.try
    yield
  rescue Failure
    NONE
  end
end
