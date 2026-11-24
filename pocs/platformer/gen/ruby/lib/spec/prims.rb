# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: the primitives Ruby's own methods do not give exactly.
# The same in every generated project; change the printer, not this file.
#
# Int is Ruby's Integer: exact at any size, so no operation wraps and none stops for width.

module Spec
  # Int operations whose domain is narrower than Integer's.
  module Ints
    module_function

    # Floored: the result has the sign of the divisor, as Integer#% does. A zero divisor fails DivByZero.
    def mod(a, b)
      Spec.fail!("DivByZero") if b.zero?
      a % b
    end

    def pow(a, b)
      Spec.stop!("E-TYPE-MISMATCH", "`^` with a negative exponent") if b.negative?
      a**b
    end

    # `/` on Int is an open choice: the program must say how to round.
    def div(_a, _b) = Spec.stop!("E-OPEN", "`/` on Int is an open choice: say how to round")

    def band(a, b) = natural(a, b) && (a & b)

    def bor(a, b) = natural(a, b) && (a | b)

    def bxor(a, b) = natural(a, b) && (a ^ b)

    def shl(a, b) = natural(a, b) && (a << b)

    def shr(a, b) = natural(a, b) && (a >> b)

    def natural(a, b)
      Spec.stop!("E-TYPE-MISMATCH", "a bit operation on a negative Int") if a.negative? || b.negative?
      true
    end
  end

  # Text is a UTF-8 String; sizes and positions count code points.
  module Texts
    # Unicode White_Space, exactly these 25 code points: not U+FEFF, U+180E or U+200B.
    WHITE_SPACE = "\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000"
    ENDS = /\A[#{WHITE_SPACE}]+|[#{WHITE_SPACE}]+\z/

    module_function

    def trim(text) = text.gsub(ENDS, "")

    # The full Unicode lower-case mapping, no locale. String#downcase gives it but for the one rule that reads
    # its neighbours: a capital sigma that ends a word is a final sigma.
    def lower(text)
      return text.downcase unless text.include?("Σ")

      chars = text.chars
      chars.each_with_index.map { |c, i| c == "Σ" ? sigma(chars, i) : c.downcase }.join
    end

    def sigma(chars, at) = cased_next?(chars[0...at].reverse) && !cased_next?(chars[(at + 1)..]) ? "ς" : "σ"

    # Whether the first character that is not case-ignorable is cased.
    def cased_next?(chars)
      found = chars.find { |c| !c.match?(/\p{Case_Ignorable}/) }
      !found.nil? && found.match?(/\p{Cased}/)
    end

    # Pieces between separators, empty ones kept. (String#split drops them and reads " " as any white space.)
    def split(text, separator)
      Spec.stop!("E-TYPE-ARITY", "`split` takes one non-empty Text") if separator.empty?
      pieces = []
      from = 0
      while (at = text.index(separator, from))
        pieces << text[from...at]
        from = at + separator.size
      end
      pieces << text[from..]
    end

    # Every non-overlapping match, left to right; the new text is taken as it is.
    def replace(text, old, new)
      Spec.stop!("E-TYPE-ARITY", "`replace` takes two Text, the first not empty") if old.empty?
      text.gsub(old) { new }
    end

    def take(text, count) = count.positive? ? text[0, count] : ""

    def drop(text, count) = count.positive? ? (text[count..] || "") : text
  end

  # List is an Array, never changed in place.
  module Lists
    module_function

    def first(list) = list.empty? ? Spec.missing! : list.first

    def last(list) = list.empty? ? Spec.missing! : list.last

    def get(list, index) = index >= 0 && index < list.size ? Some.new(list[index]) : NONE

    # `xs[i]`.
    def at(list, index) = index >= 0 && index < list.size ? list[index] : Spec.missing!

    def find(list)
      list.each { |item| return Some.new(item) if yield(item) }
      NONE
    end

    def position(list)
      list.each_with_index { |item, index| return Some.new(index) if yield(item) }
      NONE
    end

    # The least item, the earliest of ties.
    def min(list) = min_by(list) { |item| item }

    def max(list) = max_by(list) { |item| item }

    def min_by(list, &) = best(list, -1, &)

    def max_by(list, &) = best(list, 1, &)

    def best(list, sign)
      Spec.missing! if list.empty?
      found = list.first
      found_key = nil
      list.each_with_index do |item, index|
        key = yield(item)
        found, found_key = item, key if index.zero? || Spec.compare(key, found_key) == sign
      end
      found
    end

    def sort(list, desc: false) = Spec.sort_by(list, desc:) { |item| item }

    def sort_by(list, desc: false, &) = Spec.sort_by(list, desc:, &)

    def zip(left, right)
      Spec.fail!("Uneven") unless left.size == right.size
      left.each_with_index.map { |item, index| Tuple[item, right[index]] }
    end

    # Items by key, order kept inside a group; the Map's keys ascend.
    def group_by(list, &) = Maps.of(list.group_by(&).to_a)

    # `x` put before `index`, which is 0..=size.
    def insert(list, index, item)
      Spec.missing! unless index >= 0 && index <= list.size
      list.dup.insert(index, item)
    end

    def remove(list, index)
      Spec.missing! unless index >= 0 && index < list.size
      list[0, index] + list[(index + 1)..]
    end
  end

  # Set is a ::Set whose items were added in ascending order, so it iterates ascending.
  module Sets
    EMPTY = ::Set[].freeze

    module_function

    def of(items)
      return EMPTY if items.empty?

      ::Set.new(Spec.sort(items.uniq)).freeze
    end

    def add(set, item) = set.include?(item) ? set : of(set.to_a << item)

    def union(left, right) = right.subset?(left) ? left : of(left.to_a + right.to_a)
  end

  # Map is a Hash whose keys were added in ascending order, so it iterates ascending.
  module Maps
    EMPTY = {}.freeze

    module_function

    # Pairs left to right; a later equal key replaces an earlier one.
    def of(pairs)
      return EMPTY if pairs.empty?

      Spec.sort_by(pairs.to_h.to_a) { |pair| pair[0] }.to_h.freeze
    end

    def get(map, key) = map.key?(key) ? Some.new(map[key]) : NONE

    # `m[k]`.
    def at(map, key) = map.fetch(key) { Spec.missing! }

    # `m[k] = v`: replaces in place of the old entry, or adds where the key belongs.
    def put(map, key, value)
      return map.merge(key => value).freeze if map.key?(key) || map.empty? || Spec.compare(map.keys.last, key).negative?

      of(map.to_a << [key, value])
    end
  end

  # A range value is a Range of Integers: `lo..hi` or `lo...hi`.
  module Ranges
    LIMIT = 10_000_000

    module_function

    def list(range)
      Spec.stop!("E-RUNTIME", "a range of more than ten million numbers") if range.size > LIMIT
      range.to_a
    end
  end
end
