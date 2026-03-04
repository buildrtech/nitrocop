def simple_method
  x = 1
  x
end

def small_method(a, b)
  a + b
end

def empty_method
end

def one_branch
  foo.bar
end

# Repeated &. on the same local variable: only first counts as condition.
# Without discount: A=1, B=8 (8 calls), C=8 (8 safe navs) => sqrt(1+64+64)=11.36 > default 17? No.
# But with many more calls it could push over. Let's just verify it doesn't overcount.
def method_with_repeated_csend
  if (obj = find_something)
    a = obj&.foo
    b = obj&.bar
    c = obj&.baz
    d = obj&.qux
    e = obj&.quux
    f = obj&.corge
    g = obj&.grault
    h = obj&.garply
  end
end

def moderate_method
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
  l = 12
  m = 13
  n = 14
  o = 15
  p = 16
  q = 17
end

# Short define_method block should not fire
define_method(:simple_dm) do
  x = 1
  x
end

# define_method with string argument
define_method("another_dm") do
  a = 1
  b = 2
end

# Empty define_method
define_method(:empty_dm) do
end

# RuboCop ignores dynamic define_method names (dstr), so these blocks are not
# checked by Metrics/AbcSize.
define_method("dynamic_#{suffix}") do
  a = 1
  b = 2
  c = 3
  d = 4
  e = 5
  f = 6
  g = 7
  h = 8
  i = 9
  j = 10
  k = 11
  l = 12
  m = 13
  n = 14
  o = 15
  p = 16
  q = 17
  r = 18
end
