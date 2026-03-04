it do
  allow(Foo).to receive(:bar) { baz }
end
it do
  allow(Foo).to receive(:bar).and_return(42)
end
it do
  allow(Foo).to receive(:bar)
end
it do
  allow(Foo).to receive(:bar) { [42, baz] }
end
it do
  bar = 42
  allow(Foo).to receive(:bar) { bar }
end
# Constants are not static values — they can change at runtime
it do
  allow(Foo).to receive(:bar) { SomeConstant }
end
it do
  allow(Foo).to receive(:bar) { Module::CONSTANT }
end
# receive_message_chain with block is not flagged by RuboCop
it do
  allow(order).to receive_message_chain(:payments, :valid, :empty?) { false }
end
it do
  allow(obj).to receive_message_chain(:foo, :bar) { 42 }
end
