# frozen_string_literal: true

# Projected by hand from the exit codes of the cli vocabulary (docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module CLI
    module ExitCode
      OK = 0
      REFUSED = 1
      USAGE = 2
      STORE = 3
    end
  end
end
