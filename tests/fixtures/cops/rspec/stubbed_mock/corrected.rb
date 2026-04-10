allow(foo).to receive(:bar).and_return('hello world')
allow(foo).to receive(:bar) { 'hello world' }
allow(foo).to receive_messages(foo: 42, bar: 777)

canned = -> { 42 }
allow(foo).to receive(:bar, &canned).and_return(42)
