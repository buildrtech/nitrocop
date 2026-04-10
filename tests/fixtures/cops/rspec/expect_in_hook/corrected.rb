before do
  skip(something).to eq('foo')
end
after do
  skip.to eq('bar')
end
around do
  skip(Something).to receive(:foo)
end
before do
  skip { something }.to eq('foo')
end
before do
  if condition
    skip(something).to eq('bar')
  end
end
after do
  items.each do |item|
    skip(item).to be_valid
  end
end
before do
  def check_result(result)
    skip(result).to be_valid
  end
end
before do
  @validator = lambda do |val|
    skip(val).to be_present
  end
end
before do
  @items = (0..4).map do
    double("item").tap do |item|
      skip(item).to receive(:call)
    end
  end
end
before(:each) do
  skip(:response_body).and_return @body
end
after do
  skip(:cleanup)
end
