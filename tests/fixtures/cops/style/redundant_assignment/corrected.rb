def func
  some_preceding_statements
  something
end

def bar
  compute
end

def baz
  a + b
end

# Inside if/elsif/else branches
def with_if_branches
  some_preceding_statements
  if x
    1
  elsif y
    2
  else
    3
  end
end

# Inside case/when branches
def with_case_when
  some_preceding_statements
  case x
  when y
    1
  when z
    2
  else
    3
  end
end

# Inside case/in (pattern matching) branches
def with_case_in
  case x
  in y
    1
  in z
    2
  else
    3
  end
end

# Inside explicit begin..end block
def with_begin
  some_preceding_statements
  begin
    something
  end
end

# Inside rescue blocks
def with_rescue
  1
  2
rescue SomeException
  3
  4
rescue AnotherException
  5
end

# Class method (defs node)
def self.class_method
  something
end

# Inside unless branches
def with_unless
  unless condition
    1
  else
    2
  end
end
