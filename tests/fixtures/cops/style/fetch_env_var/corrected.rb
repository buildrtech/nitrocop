ENV.fetch('X', nil)
x = ENV.fetch('X', nil)
some_method(ENV.fetch('X', nil))
# Assignment in if condition: ENV['KEY'] should still be flagged
if (repo = ENV.fetch('KEY', nil))
  source(repo)
end
# ENV['X'] in && chain in condition: should be flagged (not a bare flag)
if ENV.fetch('A', nil) && ENV.fetch('B', nil) && other
  do_something
end
# case/when: both should be flagged
case ENV.fetch('X', nil)
when ENV.fetch('Y', nil)
  do_something
end
# y ||= ENV['X'] should be flagged (ENV is the value, not the receiver)
y ||= ENV.fetch('X', nil)
# y &&= ENV['X'] should be flagged
y &&= ENV.fetch('X', nil)
# y || ENV['X'] should be flagged (ENV is RHS of ||)
y || ENV.fetch('X', nil)
# Different key in body should be flagged even when condition guards another key
if ENV['X']
  puts ENV.fetch('Y', nil)
end
# ENV in condition body where condition is non-ENV
if a == b
  ENV.fetch('X', nil)
end
# Interpolation
"#{ENV.fetch('X', nil)}"
# ENV in body of &&-chain predicate condition should be flagged
if ENV['A'].present? && ENV['B'].present?
  config = ENV.fetch('A', nil)
end
# ENV in && condition chain (3+ elements): deeply nested ones flagged, direct child not
if ENV.fetch('A', nil) && ENV.fetch('B', nil) && ENV['C']
