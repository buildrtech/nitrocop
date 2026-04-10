skip 'doing x' do
  it { cool_predicate_method }
end

skip 'doing y' do
  it { cool_predicate_method }
end

skip 'when awesome case' do
  it { another_predicate_method }
end

skip 'when another awesome case' do
  it { another_predicate_method }
end

skip 'quoting case a' do
  it { expect(subject).to eq('hello') }
end

skip 'quoting case b' do
  it { expect(subject).to eq("hello") }
end

skip 'parens case a' do
  it { expect(subject).to eq(1) }
end

skip 'parens case b' do
  it { expect(subject).to eq 1 }
end

control 'test-01' do
  skip 'first check' do
    it { should eq 0 }
  end
  skip 'second check' do
    it { should eq 0 }
  end
end

if condition
  skip 'branch a' do
    it { should exist }
    it { should be_enabled }
  end
  skip 'branch b' do
    it { should exist }
    it { should be_enabled }
  end
elsif other_condition
  skip 'branch c' do
    it { should be_valid }
    it { should respond_to :name }
  end
  skip 'branch d' do
    it { should be_valid }
    it { should respond_to :name }
  end
else
  skip 'branch e' do
    it { should be_nil }
  end
  skip 'branch f' do
    it { should be_nil }
  end
end

# Negative zero vs zero — Parser gem folds -0.0 into float literal where -0.0 == 0.0
RSpec.describe 'Float#negative?' do
  skip 'on zero' do
    it { 0.0.negative?.should be_false }
  end

  skip 'on negative zero' do
    it { -0.0.negative?.should be_false }
  end
end

# Empty block params || vs no params — Parser gem treats both as empty args
RSpec.describe 'blocks' do
  skip 'taking zero arguments' do
    it { @y.z { 1 }.should == 1 }
  end

  skip 'taking || arguments' do
    it { @y.z { || 1 }.should == 1 }
  end
end
