def too_many_params(a, b, c, d, e, f)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  a + b + c + d + e + f
end

def another_long(a, b, c, d, e, f, g)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [7/5]
  [a, b, c, d, e, f, g]
end

def with_keywords(a, b, c, d, e, f:)
^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  a
end

# Block parameters should also be checked
data.each do |code, name, category, upper, lower, title|
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  process(code)
end

items.map do |a, b, c, d, e, f, g|
              ^^^^^^^^^^^^^^^^^^^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [7/5]
  a
end

records.each do |id, name, role, status, level, rank|
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  id
end

# Block with keyword params that push over the limit
items.each do |a, b, c, d, e, f:|
               ^^^^^^^^^^^^^^^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  a
end

# Initialize in a class should still be checked
class Foo
  def initialize(one:, two:, three:, four:, five:, six:)
  ^^^ Metrics/ParameterLists: Avoid parameter lists longer than 5 parameters. [6/5]
  end
end
