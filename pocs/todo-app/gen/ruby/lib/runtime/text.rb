# frozen_string_literal: true

# Runtime shim of the ruby backend of spec-lang build: Text: is-a-Text, trim. The same in every generated project; change the backend, not this file.

module Runtime
  # Text is a sequence of Unicode code points. A Ruby string carries an encoding, so "is a Text" is a check.
  module Text
    # Unicode White_Space, exactly these 25 code points: not U+FEFF, U+180E or U+200B.
    # String#strip knows only ASCII white space and NUL, so it is never used.
    WHITE_SPACE = "\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000"
    LEADING = /\A[#{WHITE_SPACE}]+/
    TRAILING = /[#{WHITE_SPACE}]+\z/

    def self.text?(value)
      value.is_a?(String) && value.encoding == Encoding::UTF_8 && value.valid_encoding?
    end

    # `Text.trim`: removes White_Space from both ends.
    def self.trim(text) = text.sub(LEADING, "").sub(TRAILING, "")
  end
end
