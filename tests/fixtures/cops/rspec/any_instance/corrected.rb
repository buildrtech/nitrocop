allow(Object).to receive(:foo)
expect(Object).to receive(:foo)
Object.any_instance.should_receive(:foo)
