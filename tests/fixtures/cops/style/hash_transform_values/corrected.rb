# each_with_object patterns
x.transform_values { |v| foo(v) }

x.transform_values { |v| v.to_s }

x.transform_values { |v| v.to_i }

x.transform_values { |v| v * 2 }

# Hash[_.map {...}] pattern
x.transform_values { |v| foo(v) }

x.transform_values { |v| v.to_s }

# _.map {...}.to_h pattern
x.transform_values { |v| v.to_s }

x.transform_values { |v| v.to_i }

# _.to_h {...} pattern
x.transform_values { |v| v.to_s }

x.transform_values { |v| foo(v) }

# ::Hash[_.map {...}] with qualified constant path
x.transform_values { |v| foo(v) }

# Multi-line map { }.to_h
x.transform_values { |klass| klass.to_s }

# map do...end.to_h
x.transform_values { |attr| attr.to_s }

# Hash[_.map do...end]
x.transform_values { |members| members.sort }

# ::Hash[_.map { }] multi-line
raw_fonts.transform_values { |font| font.to_s }
