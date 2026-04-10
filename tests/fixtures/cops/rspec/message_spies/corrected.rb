expect(foo).to have_received(:bar)
expect(foo).not_to have_received(:bar)
expect(foo).to_not have_received(:baz)
expect(foo).to have_received(:bar).with(:baz)
expect(foo).to have_received(:bar).at_most(42).times
expect(foo).to have_received(:bar).and have_received(:baz)
