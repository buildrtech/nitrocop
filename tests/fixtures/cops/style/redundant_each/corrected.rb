array.each { |v| do_something(v) }

array.each_with_index { |v| do_something(v) }

array.each_with_object([]) { |v, o| do_something(v, o) }
