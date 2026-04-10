RSpec.describe User do
  before { setup }
  it { is_expected.to be_valid }
end

RSpec.describe Post do
  after { cleanup }
  it { is_expected.to be_present }
end

RSpec.describe Comment do
  around { |test| test.run }
  context 'nested' do
    it { is_expected.to work }
  end
end
