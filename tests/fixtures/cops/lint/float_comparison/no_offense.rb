x == 0.0
x != 0.0
x == 1
x == nil
x == y
(x - 0.1).abs < Float::EPSILON
x.to_f == 0
x.to_f == 0.0
x.to_f == nil

case value
when 0.0
  foo
end

case value
when 1
  foo
when 'string'
  bar
end
