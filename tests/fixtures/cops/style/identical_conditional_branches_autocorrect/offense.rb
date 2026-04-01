if cond
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

case value
when :a
  call
  ^^^^ Style/IdenticalConditionalBranches: Move `call` out of the conditional.
else
  call
  ^^^^ Style/IdenticalConditionalBranches: Move `call` out of the conditional.
end

case value
in :a
  action
  ^^^^^^ Style/IdenticalConditionalBranches: Move `action` out of the conditional.
else
  action
  ^^^^^^ Style/IdenticalConditionalBranches: Move `action` out of the conditional.
end
