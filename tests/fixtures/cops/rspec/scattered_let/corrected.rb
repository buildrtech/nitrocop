RSpec.describe User do
  let(:a) { a }
  let(:b) { b }
  it { expect(subject.foo).to eq(a) }
end

describe Post do
  let(:x) { 1 }
  let(:y) { 2 }
  let(:z) { 3 }
  it { expect(x + y).to eq(3) }
end

describe Comment do
  let!(:a) { create(:a) }
  let!(:b) { create(:b) }
  it { expect(a).to be_valid }
end

RSpec.feature "Widgets" do
  let(:widget) { create(:widget) }
  let(:other) { create(:other) }
  it { expect(widget).to be_valid }
end

# let with &block_pass counts as a let declaration (RuboCop's `let?` matches both forms)
describe Connection do
  let(:connection) { described_class.new }
  let :fresh_connection, &NEW_PG_CONNECTION
  before { setup }
end

# block_pass let in initial group, then scattered regular let
describe Service do
  let(:handler, &HANDLER_PROC)
  let(:other) { create(:other) }
  it { expect(handler).to be_valid }
end
