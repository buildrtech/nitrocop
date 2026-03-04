RSpec.describe User do
  subject { described_class.new }

  let(:params) { foo }

  context 'nested' do
    subject { described_class.new }
    it { is_expected.to be_valid }
  end
end

RSpec.describe Post do
  subject { described_class.new }

  before { setup }
  it { is_expected.to be_present }
end

module Spree
  describe LegacyUser do
    let(:user) { create(:user) }
    before { setup }
    subject { described_class.new }
  end
end

require 'spec_helper'
module Berkshelf
  describe ChefRepoUniverse do
    let(:fixture) { nil }
    subject { described_class.new(fixture) }
  end
end

class Configuration
  describe Server do
    let(:server) { build(:server) }
    subject { described_class.new }
  end
end
