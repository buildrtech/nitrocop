# if/elsif duplicate
if condition
  do_something
elsif other
^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_something
end

# if/else duplicate
if foo
  do_foo
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
end

# unless/else duplicate
unless foo
  do_bar
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# ternary duplicate
res = foo ? do_foo : do_foo
                     ^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.

# case/when duplicate
case x
when 1
  :foo
when 2
^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  :foo
when 3
  :bar
end

# case/else duplicate
case x
when :a
  do_foo
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
end

# case with multiple duplicate whens
case x
when :a
  do_foo
when :b
  do_bar
when :c
^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
when :d
^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# if with multiple duplicate branches
if foo
  do_foo
elsif bar
  do_bar
elsif baz
^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_foo
elsif quux
^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  do_bar
end

# rescue with duplicate branches
begin
  do_something
rescue FooError
  handle_error(x)
rescue BarError
^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_error(x)
end

# rescue with else duplicate
begin
  do_something
rescue FooError
  handle_error(x)
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_error(x)
end

# rescue with multiple duplicates
begin
  do_something
rescue FooError
  handle_foo_error(x)
rescue BarError
  handle_bar_error(x)
rescue BazError
^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_foo_error(x)
rescue QuuxError
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  handle_bar_error(x)
end

# case-in (pattern matching) duplicate
case foo
in x then do_foo
in y then do_foo
^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
end

# Branches with semantically identical strings but different escape syntax are duplicates
unless "\u2028" == 'u2028'
  "{\"bar\":\"\u2028 and \u2029\"}"
else
^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  "{\"bar\":\"\342\200\250 and \342\200\251\"}"
end

# case/when branches with different whitespace but same AST
case node_type
when :dstr
  each_child(node).all? {|child| check(child)}
when :begin
^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  each_child(node).all? {|child| check(child) }
end

# if/elsif branches with different comments but same code
if foo
  # comment about foo
  handle_error(x)
  return []
elsif bar
^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  # different comment
  handle_error(x)
  return []
end

# rescue branches with different blank lines but same code
begin
  work
rescue FirstError
  report(e)
  false
rescue SecondError
^^^^^^^^^^^^^^^^^^^^ Lint/DuplicateBranch: Duplicate branch body detected.
  report(e)

  false
end
