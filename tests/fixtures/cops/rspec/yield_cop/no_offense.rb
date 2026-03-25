RSpec.describe 'test' do
  it 'allows receive with no block args' do
    allow(foo).to receive(:bar) { |block| block.call }
  end

  it 'allows block.call with extra statements' do
    allow(foo).to receive(:bar) do |&block|
      result = block.call
      transform(result)
    end
  end

  it 'uses and_yield' do
    allow(foo).to receive(:bar).and_yield
  end

  # RuboCop only flags blocks where &block is the sole parameter
  it 'allows block with extra positional params' do
    expect(controller).to receive(:before_action).with({}) { |_options, &block| block.call(controller) }
  end

  it 'allows block with extra positional params do-end' do
    allow(obj).to receive(:run) do |_arg, &block|
      block.call
    end
  end

  it 'allows block with multiple extra params' do
    allow(Dir).to receive(:chdir) { |_, &b| b.call }
  end
end
