"text"
:sym
42
x.to_s
x.to_i
# to_h with block transforms entries — not redundant
value.to_h.to_h { |k, v| [k, transform(v)] }
# Qualified constant paths are NOT the same as bare Array/Hash/String
Native::Array.new(object, *args, &block).to_a
::Native::Array.new(object).to_a
Foo::Hash.new.to_h
Custom::String.new("text").to_s
