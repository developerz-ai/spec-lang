# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: writing inside a value.
# The same in every generated project; change the printer, not this file.

module Spec
  # A step into a place: a record's member (a Symbol), a Map key or a List index.
  Key = Data.define(:key)
  Index = Data.define(:index)

  def self.key(key) = Key.new(key)

  def self.index(index) = Index.new(index)

  # `root.a[k].b = value`: the root with the value at the path. Nothing is changed in place. A Map key that is
  # the last step adds or replaces; any other missing key or index fails Missing.
  def self.put(root, path, value, at = 0)
    return value if at == path.size

    step = path[at]
    case step
    when Symbol then root.with(step => put(root.public_send(step), path, value, at + 1))
    when Key
      inner = at + 1 == path.size ? value : put(Maps.at(root, step.key), path, value, at + 1)
      Maps.put(root, step.key, inner)
    else
      list = root.dup
      list[step.index] = put(Lists.at(root, step.index), path, value, at + 1)
      list
    end
  end

  # The value at a path; a missing key or index fails Missing.
  def self.dig(root, path)
    path.reduce(root) do |value, step|
      case step
      when Symbol then value.public_send(step)
      when Key then Maps.at(value, step.key)
      else Lists.at(value, step.index)
      end
    end
  end
end
