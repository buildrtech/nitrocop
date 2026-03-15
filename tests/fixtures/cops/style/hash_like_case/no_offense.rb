case x
when 'a'
  1
when 'b'
  2
end
case x
when 'a'
  do_something
when 'b'
  do_other
when 'c'
  do_third
end
LOOKUP = { 'a' => 1, 'b' => 2, 'c' => 3 }

# Case without predicate (boolean-mode case) - not flagged
case
when x == 'a'
  1
when x == 'b'
  2
when x == 'c'
  3
end

# Case with else clause - can't trivially replace with hash
case x
when 'a'
  'first'
when 'b'
  'second'
when 'c'
  'third'
else
  'default'
end

# Integer conditions - RuboCop only allows str/sym conditions
case value.length
when 32
  "md5"
when 40
  "sha1"
when 64
  "sha256"
end

# Nil body disqualifies the whole case
case command
when :selrange
  nil
when :foo
  :bar
when :baz
  :qux
end

# Mixed true/false bodies (different AST types)
case mode
when :synchronous
  true
when :buffered
  false
when :async
  true
end

# Mixed int/float bodies (different AST types)
case unit
when :float_second
  1_000_000_000.0
when :second
  1_000_000_000
when :millisecond
  1_000_000
end

# Mixed condition types (string and symbol)
case x
when 'foo'
  'FOO'
when :bar
  'BAR'
when 'baz'
  'BAZ'
end
