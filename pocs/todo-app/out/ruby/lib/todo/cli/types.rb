# frozen_string_literal: true

# Projected by hand from type TaskNo, Title, Tag, Priority as argument types (src/todo/model.slg, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../model/title"
require_relative "../model/tag"
require_relative "../model/priority"

module Todo
  module CLI
    # One argument's text to a value of the type, or nil when the text is not of the type.
    module Types
      module TaskNo
        NAME = "TaskNo"
        DIGITS = /\A[0-9]+\z/

        # ASCII digits only; leading zeros allowed; any length, Integer being exact.
        def self.parse(text)
          return unless text.valid_encoding? && text.match?(DIGITS)

          value = Integer(text, 10)
          value if value >= 1
        end
      end

      module Title
        NAME = "Title"

        def self.parse(text) = (text if Model.title?(text))
      end

      module Tag
        NAME = "Tag"
        PLACEHOLDER = "<tag>"

        def self.parse(text) = (text if Model.tag?(text))
      end

      # A sum type with no data: its variant names in lower case.
      module Priority
        NAME = "Priority"
        VARIANTS = Model::Priority::ALL.to_h { |variant| [variant.name.downcase, variant] }.freeze
        PLACEHOLDER = VARIANTS.keys.join("|").freeze

        def self.parse(text) = VARIANTS[text]
      end
    end
  end
end
