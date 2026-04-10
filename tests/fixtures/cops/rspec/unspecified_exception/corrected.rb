RSpec.describe User do
  it 'raises an error' do
    expect { raise StandardError }.to raise_error(StandardError)
  end

  it 'raises an exception' do
    expect { raise StandardError }.to raise_exception(StandardError)
  end

  it 'chains' do
    expect { foo }.to raise_error(StandardError).and change { bar }
  end
end
