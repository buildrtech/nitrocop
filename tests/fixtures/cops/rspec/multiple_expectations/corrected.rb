RSpec.describe Foo do
  it 'uses expect twice' do
    skip('TODO: split expectations')
  end

  it 'uses is_expected twice' do
    skip('TODO: split expectations')
  end

  it 'uses expect with blocks' do
    skip('TODO: split expectations')
  end

  # should-style expectations (implicit subject)
  it 'uses should twice' do
    skip('TODO: split expectations')
  end

  it 'uses should_not twice' do
    skip('TODO: split expectations')
  end

  it 'uses are_expected twice' do
    skip('TODO: split expectations')
  end

  it 'uses should_receive twice' do
    skip('TODO: split expectations')
  end

  it 'uses should_not_receive twice' do
    skip('TODO: split expectations')
  end

  it 'mixes expect and should' do
    skip('TODO: split expectations')
  end
end

# focus is a focused example alias (like fit/fspecify)
  focus 'uses expect twice with focus' do
    skip('TODO: split expectations')
  end
end

# pending used as example group wrapper — nested examples still checked
pending 'deferred feature' do
  it 'has too many expectations in pending group' do
    skip('TODO: split expectations')
  end
end

# skip used as example group wrapper — nested examples still checked
skip 'disabled feature' do
  it 'has too many expectations in skip group' do
    skip('TODO: split expectations')
  end
end

# aggregate_failures: false overrides inherited aggregate_failures
describe Foo, aggregate_failures: true do
  it 'overrides with false', aggregate_failures: false do
    skip('TODO: split expectations')
  end
end
