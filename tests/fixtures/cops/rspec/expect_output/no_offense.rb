specify do
  $stdout.puts("hi")
end

specify do
  $blah = StringIO.new
end

it 'uses output matcher' do
  expect { run }.to output("hello").to_stdout
end

# $stdout/$stderr in method definitions are NOT flagged
def capture_output
  $stdout = StringIO.new
  $stderr = StringIO.new
end

# $stdout/$stderr in before(:all) hooks are NOT flagged
before(:all) do
  $stdout = StringIO.new
end

# $stdout/$stderr in before(:context) hooks are NOT flagged
before(:context) do
  $stderr = StringIO.new
end

# $stdout/$stderr in before(:suite) hooks are NOT flagged
before(:suite) do
  $stderr = StringIO.new
end

# $stdout/$stderr in describe block (example group scope) are NOT flagged
describe 'something' do
  $stdout = StringIO.new
end

# $stdout/$stderr at root scope are NOT flagged
$stderr = StringIO.new

# Multi-write at root scope is NOT flagged
@old, $stdout = $stdout, StringIO.new

# Multi-write in method definition is NOT flagged
def swap_streams
  @old, $stderr = $stderr, StringIO.new
end

# Multi-write in before(:all) is NOT flagged
before(:all) do
  @old, $stdout = $stdout, StringIO.new
end
