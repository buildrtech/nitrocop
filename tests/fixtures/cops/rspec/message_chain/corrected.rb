allow(foo).to skip(:one, :two)
allow(bar).to skip(:a, :b, :c)
foo.skip(:one, :two).and_return(:three)
