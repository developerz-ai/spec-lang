# frozen_string_literal: true

# Projected by hand from the usage errors of the cli vocabulary (docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module CLI
    # The arguments are not a call of any rule. The message is the line after "error: ".
    class UsageError < StandardError; end
  end
end
