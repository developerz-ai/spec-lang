# frozen_string_literal: true

# Projected by hand from fn is_title and fn is_tag, transcribed word for word as the oracle for the optimized forms (src/todo/model.slg, docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module TestSupport
    # The spec's formulas as written, slow and literal. The product code must agree with them on every text.
    module SpecOracle
      WHITE_SPACE = [*0x0009..0x000D, 0x0020, 0x0085, 0x00A0, 0x1680, *0x2000..0x200A, 0x2028, 0x2029, 0x202F, 0x205F,
                     0x3000].freeze

      def self.trim(text)
        points = text.codepoints
        points.shift while WHITE_SPACE.include?(points.first)
        points.pop while WHITE_SPACE.include?(points.last)
        points.pack("U*")
      end

      # t.trim().size >= 1 and t.size <= 200 and t.bytes.all(it >= 32 and it != 127)
      # and not t.contains("\u{0085}") and not t.contains("\u{2028}") and not t.contains("\u{2029}")
      def self.title?(text)
        trim(text).size >= 1 && text.size <= 200 && text.bytes.all? { |it| it >= 32 && it != 127 } &&
          !text.include?("\u0085") && !text.include?("\u2028") && !text.include?("\u2029")
      end

      # 1 <= t.size <= 32 and t.bytes.all(97 <= it <= 122 or 48 <= it <= 57 or it == 45) and not t.starts_with("-")
      def self.tag?(text)
        text.size.between?(1, 32) &&
          text.bytes.all? { |it| it.between?(97, 122) || it.between?(48, 57) || it == 45 } &&
          !text.start_with?("-")
      end
    end
  end
end
