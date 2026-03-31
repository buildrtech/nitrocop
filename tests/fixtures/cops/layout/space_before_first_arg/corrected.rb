puts "hello"
puts "hello", "world"
foo "bar"

# Extra spaces without alignment on adjacent lines are offenses
something x
something y, z

# Extra space with receiver
a.something y, z

# Extra space with safe navigation
a&.something y, z

# Extra spaces not aligned with anything on adjacent lines
describe "with http basic auth features" do
end

# has_many/belongs_to with extra spaces not aligned
has_many :security_groups

# Vertical argument position NOT aligned (no_space case)
obj = a_method(arg, arg2)
obj.no_parenthesized 'asdf'

# Extra spaces with tabs in the gap should still be flagged
method :arg

# The 2nd nearest non-blank line should not cause false alignment.
# RuboCop only checks the nearest non-blank line in pass 1 and the nearest
# same-indent line in pass 2. Here the nearest non-blank line (x :y) does NOT
# have a \s\S boundary at col 12, but the 2nd line (xxxxxxxxxxz :thing) does.
# Nitrocop should NOT use the 2nd line for alignment — it should flag this.
xxxxxxxxxxz :thing
x :y
short :sym
