hash = { a: 1, b: 2, c: 3 }
hash = { 'x' => 1, 'y' => 2, 'z' => 3 }
hash = { 1 => :a, 2 => :b, 3 => :c }
hash = {}
hash = { foo: 'bar' }
hash = { a: 1, **other }

# Method call keys are not literal duplicates
{ generate_id => "a", generate_id => "b" }
{ Time.now => "first", Time.now => "second" }
{ counter += 1 => "a", counter += 1 => "b" }
{ some_method_call(x, y) => 1, some_method_call(x, y) => 4 }
