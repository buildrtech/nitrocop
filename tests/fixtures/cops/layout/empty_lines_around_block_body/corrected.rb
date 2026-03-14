items.each do |x|
  puts x
end
[1, 2].map do |x|
  x * 2
end
foo.select do |x|
  x > 1
end
make_routes = -> (a) {
  a.map { |c| c.name }
}
action = -> () {
  do_something
}
handler = -> (opts = {}) {
  opts.reduce({}) do |memo, k|
    memo
  end
}
it 'always yields if forced to, even after the initial yield or if the period ' \
   'has not passed' do
  throttle = ProgressBar::Throttle.new(:throttle_rate => 10)
end
describe 'some behavior that requires a very long description to explain ' \
         'what is being tested' do
  it 'works correctly' do
    expect(true).to be true
  end
end
