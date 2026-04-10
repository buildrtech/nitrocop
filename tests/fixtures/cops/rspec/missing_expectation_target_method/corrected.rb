RSpec.describe 'test' do
  it 'something' do
    something = 1
    expect(something).to Integer
  end

  it 'uses eq? directly' do
    expect(something).to 42
  end

  it 'uses == with is_expected' do
    is_expected to 42
  end
end
