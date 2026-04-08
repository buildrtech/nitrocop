describe Foo do
  it 'uses expect incorrectly' do
    expect(bar).to eq(123)
    expect(bar).to eq(true)
    expect(bar).to eq("foo")
    expect(bar).to eq(nil)
    expect(bar).to eq(:sym)
    expect(expected_path).to eq(__FILE__)
  end
end
