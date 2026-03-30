x.transform_keys { |k| foo(k) }

x.transform_keys { |k| k.to_sym }

x.transform_keys { |k| k.to_s }
