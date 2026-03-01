[1, 2, 3].include?(e)
{ foo: :bar }.key?(:foo)
loop do
  [1, nil, 3].compact!
end
loop do
  { foo: :bar, baz: nil }.select! { |_k, v| !v.nil? }
end
loop do
  array = [1, nil, 3]
end
loop do
  hash = { foo: :bar, baz: nil }
end
[1, 2, variable].include?(e)
{ foo: { bar: variable } }.key?(:foo)
[[1, 2, 3] | [2, 3, 4]].each { |e| puts e }
loop do
  array.all? { |x| x > 100 }
end
while x < 100
  puts x
end
# Safe navigation calls are not loops
items&.each do |item|
  [1, 2, 3].include?(item)
end
pipeline["steps"]&.each do |step|
  %w(TEST TESTS TESTOPT TESTOPTS).each do |env_var|
    puts env_var
  end
end
# Literal that structurally equals the receiver of an ancestor loop
[1].each { [1].each { puts 1 } }
[1, 2, 3].each { |i| [1, 2, 3].each { |j| puts j } }
[true, false].each do |flag1|
  [true, false].each do |flag2|
    do_something(flag1, flag2)
  end
end
%i[subscribe assign].each do |mode_name|
  other_modes = %i[subscribe assign] - [mode_name]
end
[:install, "install", :gem, "gem"].each do |type|
  ([:install, "install", :gem, "gem"] - [type]).each do |other|
    puts other
  end
end
