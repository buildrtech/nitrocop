def m
  items.something { |i| yield i }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def n
  items.each { |x| yield x }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def o
  foo.bar { |a, b| yield a, b }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def p
  3.times { yield }
  ^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def q
  super { yield }
  ^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end
