expect(foo).not_to receive(:bar)

expect(foo).not_to receive(:bar).with(1)

expect(foo).not_to receive(:baz)
