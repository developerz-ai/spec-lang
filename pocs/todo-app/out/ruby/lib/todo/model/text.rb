# frozen_string_literal: true

# Projected by hand from Text, its size and trim, as fn is_title and is_tag use them (src/todo/model.slg, docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Model
    # Text is a sequence of Unicode code points. A Ruby string carries an encoding, so "is a Text" is a check.
    module Text
      # Unicode White_Space, exactly the 25 code points of the contract: not U+FEFF, U+180E or U+200B.
      # String#strip knows only ASCII white space and NUL, so it is never used.
      WHITE_SPACE = "\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000"

      # `t.trim().size >= 1` holds exactly when one code point is not white space: no trimmed copy is built.
      NOT_WHITE_SPACE = /[^#{WHITE_SPACE}]/

      def self.text?(value)
        value.is_a?(String) && value.encoding == Encoding::UTF_8 && value.valid_encoding?
      end

      def self.blank?(text) = !text.match?(NOT_WHITE_SPACE)
    end
  end
end
