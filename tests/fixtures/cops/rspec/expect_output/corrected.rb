specify do
  skip('TODO: use output matcher')
end

before(:each) do
  skip('TODO: use output matcher')
end

it 'captures output' do
  skip('TODO: use output matcher')
end

# Assignment inside rescue block within an example
it 'handles rescue' do
  begin
    run_something
  rescue StandardError
    skip('TODO: use output matcher')
  end
end

# Assignment inside ensure block within an example
it 'handles ensure' do
  begin
    skip('TODO: use output matcher')
  ensure
    skip('TODO: use output matcher')
  end
end

# Assignment inside conditional within an example
it 'handles if' do
  if ENV['CAPTURE']
    skip('TODO: use output matcher')
  end
end

# Assignment inside a before hook (default scope is :each)
before do
  skip('TODO: use output matcher')
end

# Assignment inside an around hook
around do |example|
  skip('TODO: use output matcher')
  example.run
  skip('TODO: use output matcher')
end

# Multi-write with $stdout as target
it 'reassigns stdout via multi-write' do
  skip('TODO: use output matcher')
  skip('TODO: use output matcher')
end

# Multi-write with $stderr as target
it 'reassigns stderr via multi-write' do
  skip('TODO: use output matcher')
  skip('TODO: use output matcher')
end

# Multi-write with $stdout as first target
it 'reassigns stdout as first target' do
  skip('TODO: use output matcher')
end

# Assignment inside an around hook nested in a method definition
def capture_output!(variable)
  around do |example|
    @captured_stream = StringIO.new
    original_stream = $stdout
    skip('TODO: use output matcher')
    example.run
    skip('TODO: use output matcher')
  end
end
