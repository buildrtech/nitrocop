# Self-assigning else branch in ternary
foo = condition ? bar : foo
                        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.

# Self-assigning if branch in ternary
foo = condition ? foo : bar
                  ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.

# Self-assigning else branch in if/else
foo = if condition
        bar
      else
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      end

# Self-assigning if branch in if/else
foo = if condition
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      else
        bar
      end

# Self-assigning else branch with empty if branch
foo = if condition
      else
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      end

# Self-assigning if branch with empty else branch
foo = if condition
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      else
      end
