describe 'doing x' do
  it 'TODO: unique description' do
  end

  it 'TODO: unique description' do
  end
end

describe 'doing y' do
  it 'TODO: unique description' do
  end

  context 'during some use case' do
    it "does y" do
    end
  end

  it 'TODO: unique description' do
  end
end

# Different quote styles should still match (same content)
describe 'quote normalization' do
  it 'TODO: unique description' do
  end

  it 'TODO: unique description' do
  end
end

describe 'iterator examples' do
  %i[foo bar].each do |type|
    it 'TODO: unique description' do
    end

    it 'TODO: unique description' do
    end
  end
end
