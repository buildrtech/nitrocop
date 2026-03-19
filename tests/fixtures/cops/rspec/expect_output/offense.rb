specify do
  $stdout = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end

before(:each) do
  $stderr = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
end

it 'captures output' do
  $stdout = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end

# Assignment inside rescue block within an example
it 'handles rescue' do
  begin
    run_something
  rescue StandardError
    $stderr = StringIO.new
    ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
  end
end

# Assignment inside ensure block within an example
it 'handles ensure' do
  begin
    $stdout = StringIO.new
    ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
  ensure
    $stdout = STDOUT
    ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
  end
end

# Assignment inside conditional within an example
it 'handles if' do
  if ENV['CAPTURE']
    $stdout = StringIO.new
    ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
  end
end

# Assignment inside a before hook (default scope is :each)
before do
  $stdout = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end

# Assignment inside an around hook
around do |example|
  $stderr = StringIO.new
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
  example.run
  $stderr = STDERR
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
end

# Multi-write with $stdout as target
it 'reassigns stdout via multi-write' do
  @old, $stdout = $stdout, StringIO.new
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
  $stdout = @old
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end

# Multi-write with $stderr as target
it 'reassigns stderr via multi-write' do
  @old, $stderr = $stderr, StringIO.new
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
  $stderr = @old
  ^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stderr` instead of mutating $stderr.
end

# Multi-write with $stdout as first target
it 'reassigns stdout as first target' do
  $stdout, @old = StringIO.new, $stdout
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectOutput: Use `expect { ... }.to output(...).to_stdout` instead of mutating $stdout.
end
