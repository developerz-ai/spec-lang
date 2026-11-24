# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: the one value order.
# The same in every generated project; change the printer, not this file.

module Spec
  # `false < true`; numbers by value; Text by code point (a prefix first); records field by field; sums by
  # declared variant, then fields; lists and tuples item by item; sets and maps as their ascending sequences.
  # Every sort, min, max, Set and Map uses it. Returns -1, 0 or 1.
  def self.compare(a, b)
    case a
    when Integer, String then a <=> b # UTF-8 bytes order as code points do
    when Array then compare_items(a, b)
    when true, false then a == b ? 0 : (a ? 1 : -1)
    when ::Set then compare_items(a.to_a, b.to_a)
    when Hash then compare_items(a.to_a, b.to_a)
    when nil then 0
    when Some then b.is_a?(Some) ? compare(a.value, b.value) : -1
    when None then b.is_a?(None) ? 0 : 1
    else compare_other(a, b)
    end
  end

  def self.compare_items(a, b)
    a.each_with_index do |item, index|
      return 1 if index >= b.size

      order = compare(item, b[index])
      return order unless order.zero?
    end
    a.size <=> b.size
  end

  def self.compare_other(a, b)
    case a
    when Data then compare_data(a, b)
    when Sum::Constant then a.spec_tag <=> b.spec_tag
    when ::Range then compare_items([a.begin, a.end, a.exclude_end? ? 0 : 1], [b.begin, b.end, b.exclude_end? ? 0 : 1])
    when Failure then compare_items([a.name, a.data], [b.name, b.data])
    else Spec.stop!("E-TYPE-MISMATCH", "#{show(a)} has no order")
    end
  end

  def self.compare_data(a, b)
    case a
    when Sum::Fielded
      order = a.spec_tag <=> b.spec_tag
      order.zero? ? compare_items(a.spec_fields, b.spec_fields) : order
    when Ok then b.is_a?(Ok) ? compare(a.value, b.value) : -1
    when Err then b.is_a?(Err) ? compare(a.error, b.error) : 1
    when Event then compare_items([a.rule.id, a.args], [b.rule.id, b.args])
    else compare_items(a.deconstruct, b.deconstruct)
    end
  end

  # Ascending and stable: equal items keep their order.
  def self.sort(items) = sort_by(items) { |item| item }

  # Ascending by the block's key, stable. `desc` reverses the order, never the ties.
  def self.sort_by(items, desc: false)
    sign = desc ? -1 : 1
    keyed = items.each_with_index.map { |item, index| [yield(item), index, item] }
    keyed.sort! do |x, y|
      order = compare(x[0], y[0]) * sign
      order.zero? ? x[1] <=> y[1] : order
    end
    keyed.map { |entry| entry[2] }
  end
end
