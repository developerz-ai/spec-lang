# frozen_string_literal: true

# Projected by hand from fn is_tag and type Tag (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "text"

module Todo
  module Model
    # 1 to 32 code points, each of a-z, 0-9 or "-", the first not "-": every one is a single byte, so one
    # pattern is the whole claim.
    TAG = /\A[a-z0-9][a-z0-9-]{0,31}\z/

    # is_tag: a tag is typed in a shell without quotes, and never reads as a flag.
    def self.tag?(text)
      Text.text?(text) && text.match?(TAG)
    end
  end
end
