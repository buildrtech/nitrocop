items.each do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  puts x

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body end.
end
[1, 2].map do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  x * 2

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body end.
end
foo.select do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  x > 1
end
make_routes = -> (a) {

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  a.map { |c| c.name }
}
action = -> () {

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  do_something

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body end.
}
handler = -> (opts = {}) {

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  opts.reduce({}) do |memo, k|
    memo
  end
}
it 'always yields if forced to, even after the initial yield or if the period ' \
   'has not passed' do

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  throttle = ProgressBar::Throttle.new(:throttle_rate => 10)
end
describe 'some behavior that requires a very long description to explain ' \
         'what is being tested' do

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  it 'works correctly' do
    expect(true).to be true
  end
end
