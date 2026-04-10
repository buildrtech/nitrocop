it 'checks the subject' do
  expect(subject).to be_good
end
it 'checks negation' do
  expect(subject).to be_good
end
it 'checks should_not' do
  expect(subject).not_to be_bad
end
