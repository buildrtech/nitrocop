RSpec.describe Foo do
  it { skip('TODO: add expectation') }

  specify { skip('TODO: add expectation') }

  it 'does nothing useful' do
    skip('TODO: add expectation')
  end

  # Bacon-style .should with a receiver is NOT an expectation (requires receiver-less call)
  it 'uses bacon style should' do
    skip('TODO: add expectation')
  end
end
