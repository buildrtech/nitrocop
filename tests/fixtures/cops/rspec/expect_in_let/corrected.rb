let(:foo) do
  skip(something).to eq 'foo'
end
let(:bar) do
  skip.to eq 'bar'
end
let(:baz) do
  skip(Something).to receive :foo
end
let(:nested_block) do
  items.each { |i| skip(i).to be_valid }
end
let(:conditional) do
  if condition
    skip(value).to eq(1)
  end
end
let(:ternary) do
  condition ? skip(value).to(eq(1)) : nil
end
let(:logical_and) do
  valid && skip(result).to(be_truthy)
end
let(:rescue_block) do
  begin
    skip(something).to eq(1)
  rescue StandardError
    nil
  end
end
let(:nested_expect) do
  skip do
    skip do
      DummiesIndex.bulk body: [{index: {_id: 42}}]
    end.not_to update_index(DummiesIndex)
  end
end
let :symbol_syntax do
  skip(value).to eq(1)
end
let :rescue_symbol do
  begin
    fail "something went wrong"
  rescue => error
    skip(error).to receive(:backtrace).and_return([])
    error
  end
end
let(:with_def) do
  Class.new(Base) do
    include RSpec::Matchers
    def visit_me
      skip(location).to eq '/visit_me'
    end
  end
end
let!(:with_def_bang) do
  Class.new(Base) do
    include RSpec::Matchers
    def check_value
      skip(self).to respond_to :something
    end
  end
end
