it 'something' do
  expect(something).to be 1
end
it 'something' do
  expect(something).not_to eq(2)
end
it 'something' do
  expect { something }.to raise_error(StandardError)
end
it 'something' do
  MyObject.expect(:foo)
end
it 'parenthesized expect with to' do
  (expect something).to be 1
end
it 'parenthesized expect with not_to' do
  (expect something).not_to eq(2)
end
it 'parenthesized expect with to_not' do
  (expect something).to_not eq(3)
end
