obj.then { |x| x.do_something }

1.then { |x| x + 1 }

foo.then(&method(:bar))
