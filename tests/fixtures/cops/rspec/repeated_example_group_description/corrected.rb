describe 'TODO: unique group description' do
  it { something }
end

describe 'TODO: unique group description' do
  it { other }
end

context 'TODO: unique group description' do
  it { thing }
end

context 'TODO: unique group description' do
  it { other_thing }
end

# Repeated groups inside a module wrapper (FN case)
module MyModule
  describe 'TODO: unique group description' do
    it { works }
  end

  describe 'TODO: unique group description' do
    it { also_works }
  end
end

# Repeated groups inside a class wrapper
class MySpec
  context 'TODO: unique group description' do
    it { passes }
  end

  context 'TODO: unique group description' do
    it { also_passes }
  end
end

# Quote style difference (double vs single) should still flag as duplicate
describe 'TODO: unique group description' do
  it { something }
end

describe 'TODO: unique group description' do
  it { other }
end
