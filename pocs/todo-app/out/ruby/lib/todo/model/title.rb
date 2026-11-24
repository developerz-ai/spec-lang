# frozen_string_literal: true

# Projected by hand from fn is_title and type Title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "text"

module Todo
  module Model
    TITLE_MAX_SIZE = 200
    # A byte below 32, or 127 (bytes of a multi-byte code point are all above 127), or one of the three
    # line breaks above them: next line, line separator, paragraph separator. One scan finds any of them.
    TITLE_FORBIDDEN = /[\u0000-\u001F\u007F\u0085\u2028\u2029]/

    # is_title: a title fits on one line of a terminal list, so it holds no line break of any kind.
    # Size counts code points, and no text has more code points than bytes, so the byte count answers first.
    def self.title?(text)
      Text.text?(text) &&
        (text.bytesize <= TITLE_MAX_SIZE || text.size <= TITLE_MAX_SIZE) &&
        !text.match?(TITLE_FORBIDDEN) &&
        !Text.blank?(text)
    end
  end
end
