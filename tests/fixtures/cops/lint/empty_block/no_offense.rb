items.each { |x| puts x }

items.each do |x|
  puts x
end

foo { bar }

# Block with only a comment (AllowComments default true)
items.each do |x|
  # TODO: implement
end

# Inline empty block with trailing comment on same line
Mail.connection {} # rubocop:disable:block
foo.bar {} # some explanation comment

# Empty lambdas and procs (AllowEmptyLambdas default true)
lambda {}
lambda do
end
proc {}
proc do
end

# Proc.new is treated as a proc (AllowEmptyLambdas default true)
Proc.new {}
Proc.new do
end
::Proc.new {}
::Proc.new do
end

# Chained empty block where comments exist in earlier block of the expression
# RuboCop checks the full block expression range for comments (AllowComments)
Context.create_table(:users) do |t|
  # Create timestamps.
  t.timestamps null: false
end.define_model do
end
