def complex_method
^^^ Metrics/AbcSize: Assignment Branch Condition size for complex_method is too high. [18.00/17]
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

def branchy_method(x)
^^^ Metrics/AbcSize: Assignment Branch Condition size for branchy_method is too high. [18.38/17]
  a = x.foo
  b = x.bar
  c = x.baz
  d = x.qux
  e = x.quux
  f = x.corge
  g = x.grault
  h = x.garply
  i = x.waldo
  j = x.fred
  k = x.plugh
  l = x.xyzzy
  m = x.thud
end

def mixed_method(x)
^^^ Metrics/AbcSize: Assignment Branch Condition size for mixed_method is too high. [17.15/17]
  a = x.foo
  b = x.bar
  c = x.baz
  d = x.qux
  e = x.quux
  f = x.corge
  g = x.grault
  h = x.garply
  i = x.waldo
  j = x.fred
  k = x.plugh
  if a
    l = 1
  end
  if b
    m = 1
  end
end

# define_method blocks are treated as method definitions for ABC scoring
define_method(:complex_dm) do
^^^ Metrics/AbcSize: Assignment Branch Condition size for complex_dm is too high. [18.00/17]
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

# []= setter calls count as assignments and branches in RuboCop's ABC metric.
def indexed_assignment_heavy
^^^ Metrics/AbcSize: Assignment Branch Condition size for indexed_assignment_heavy is too high. [17.69/17]
  hash = {}
  hash[:a] = 1
  hash[:b] = 2
  hash[:c] = 3
  hash[:d] = 4
  hash[:e] = 5
  hash[:f] = 6
  hash[:g] = 7
  hash[:h] = 8
  hash[:i] = 9
  hash[:j] = 10
  hash[:k] = 11
  hash[:l] = 12
end

# Multi-assignment targets each count as an assignment in RuboCop.
# a, b, c, ... = x.split('|') => each target is an assignment.
# A=18 (targets), B=1 (split), C=0 => score = sqrt(324+1) = 18.03
def multi_write_method(data)
^^^ Metrics/AbcSize: Assignment Branch Condition size for multi_write_method is too high. [18.03/17]
  a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r = data.split("|")
end

# rescue => var counts as an assignment in RuboCop (lvasgn in Parser AST).
# A=17 (a..p + err), B=1 (foo), C=1 (rescue) => sqrt(289+1+1) = 17.06
def method_with_rescue_var
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_rescue_var is too high. [17.06/17]
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
  begin
    foo
  rescue => err
  end
end

# Lambda literals (-> {}) count as a branch in RuboCop.
# In Parser AST, -> {} is (block (send nil :lambda) ...) and the :lambda send
# counts as B+1. In Prism, -> {} is LambdaNode with no CallNode.
# A=13, B=13 (lambda implicit calls), C=0 => sqrt(169+169) = 18.38
def method_with_many_lambdas
^^^ Metrics/AbcSize: Assignment Branch Condition size for method_with_many_lambdas is too high. [18.38/17]
  a = -> {}
  b = -> {}
  c = -> {}
  d = -> {}
  e = -> {}
  f = -> {}
  g = -> {}
  h = -> {}
  i = -> {}
  j = -> {}
  k = -> {}
  l = -> {}
  m = -> {}
end
