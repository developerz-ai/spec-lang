module Runtime
  # A value outside its type reached a place of that type. Edges check first, so this is a bug in the program.
  class ClaimViolation < ArgumentError; end

  # A rule refused. Raised before any new world exists, so the world is unchanged.
  class Refusal < StandardError
    def self.error_name = name.split("::").last

    def initialize(message = self.class.error_name) = super

    def error_name = self.class.error_name
  end

  # What a rule that did not refuse gives back. Lines are sent by the caller, after commit.
  Outcome = Data.define(:world, :lines, :reply)

  # A base value entering a claimed type, inside the program: the value, checked.
  def self.claim(type, value)
    raise ClaimViolation, "not a #{type::NAME}: #{value.inspect}" unless type.valid?(value)

    value
  end

  # Maps the store read for one rule call. Nothing else holds them, so the rule writes them in place.
  OWNED = {}.compare_by_identity
  private_constant :OWNED

  # Runs the block (the store reading a world); every unfrozen Hash a World takes in it is owned.
  def self.adopting
    @adopting = true
    yield
  ensure
    @adopting = false
  end

  # A value held by a record or a world: frozen, and not shared with a caller that could change it.
  # A Hash read by the store for this one call is kept as it is, to be the rule's draft.
  def self.frozen(value)
    return value if value.frozen?

    if @adopting && value.is_a?(Hash)
      OWNED[value] = true
      return value
    end
    value.dup.freeze
  end

  # The map a rule body writes. A rule is atomic and the process commits once, so a map the store
  # handed over is the draft itself; any other caller keeps its world and the draft is a copy.
  def self.draft(map) = OWNED.delete(map) ? map : map.dup

  # `map[key] = value`. A Map iterates in ascending key order and a Hash in insertion order, so the two
  # are kept equal: a key above every other is appended, any other new key re-sorts.
  def self.put(map, key, value)
    fits = map.key?(key) || map.empty? || (map.keys.last <=> key).negative?
    map[key] = value
    map.replace(map.sort.to_h) unless fits
    map
  end

  # `Map(k -> v, ...)`, in ascending key order.
  def self.map_of(map) = map.sort.to_h.freeze

  # `xs.sort(key)`: ascending by key, stable (Enumerable#sort_by is not).
  def self.sort_by(items)
    items.each_with_index.sort_by { |item, index| [yield(item), index] }.map(&:first)
  end
end
