# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: a value as spec-lang source.
# The same in every generated project; change the printer, not this file.

module Spec
  # What `"{x}"` prints for anything but Text and Int: `[1, 2]`, `Set(1, 2)`, `Map(1 -> "a")`, `(1, "a")`,
  # `Task(1, title: "milk")`, `Held(by, until: 3)`, `Some(1)`, `none`, `1..=5`, `fail Name(data)`.
  def self.show(value)
    case value
    when String then quote(value)
    when Integer, true, false then value.to_s
    when nil then "()"
    when Tuple then "(#{value.map { |item| show(item) }.join(", ")})"
    when Array then "[#{value.map { |item| show(item) }.join(", ")}]"
    when ::Set then "Set(#{value.map { |item| show(item) }.join(", ")})"
    when Hash then "Map(#{value.map { |key, item| "#{show(key)} -> #{show(item)}" }.join(", ")})"
    else show_other(value)
    end
  end

  def self.show_other(value)
    case value
    when Some then "Some(#{show(value.value)})"
    when None then "none"
    when Sum::Constant then value.variant_name
    when Sum::Fielded then "#{value.variant_name}(#{show_fields(value.class.field_names, value.spec_fields)})"
    when ::Range then "#{value.begin}#{value.exclude_end? ? "..<" : "..="}#{value.end}"
    when Proc then "<function>"
    when Failure then value.data.empty? ? "fail #{value.name}" : "fail #{value.name}(#{value.data.map { |x| show(x) }.join(", ")})"
    else show_data(value)
    end
  end

  def self.show_data(value)
    case value
    when Ok then "Ok(#{show(value.value)})"
    when Err then "Err(#{show(value.error)})"
    when Event then "#{value.rule.short}(#{show_fields(value.rule.params.map(&:name), value.args)})"
    when Step then "#{show(value.event)} -> #{show(value.reply)}"
    when Trace then "<trace of #{value.steps.size} steps>"
    else
      type = value.class
      if type.respond_to?(:spec_name) then "#{type.spec_name}(#{show_fields(type.field_names, value.deconstruct)})"
      elsif type.respond_to?(:state_names) then "World(#{type.state_names.zip(value.deconstruct).map { |name, x| "#{name}: #{show(x)}" }.join(", ")})"
      else value.inspect
      end
    end
  end

  # The first field by position, the rest by name.
  def self.show_fields(names, values)
    values.each_with_index.map { |x, i| i.zero? || names[i].nil? ? show(x) : "#{names[i]}: #{show(x)}" }.join(", ")
  end

  # Unicode White_Space other than the space, and every control character.
  ESCAPED = /[\p{Cc}\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]/
  NEEDS_QUOTING = /["\\{]|#{ESCAPED.source}/

  def self.quote(text)
    return "\"#{text}\"" unless text.match?(NEEDS_QUOTING)

    body = text.gsub(NEEDS_QUOTING) do |c|
      case c
      when '"' then '\\"'
      when "\\" then "\\\\"
      when "\n" then "\\n"
      when "\t" then "\\t"
      when "{" then "\\{"
      else format("\\u{%X}", c.ord)
      end
    end
    "\"#{body}\""
  end
end
