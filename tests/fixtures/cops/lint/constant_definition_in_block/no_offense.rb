FILES_TO_LINT = Dir['lib/*.rb']

class TestRequest; end

LIST = []

class TestEnum < T::Enum
  enums do
    Foo = new("foo")
  end
end

module M
  CONSTANT = 1
end

# Namespaced constant writes in blocks are intentional (explicit scope)
config.before_configuration do
  ::REDIS_CONFIGURATION = RedisConfiguration.new
end

task :setup do
  Module::SETTING = true
end

# Constant inside an if/unless inside a block is NOT a direct child of the block
# RuboCop does not flag this — the if breaks the direct parent relationship
describe 'config' do
  if DOORKEEPER_ORM == :active_record
    class FakeCustomModel < ::ActiveRecord::Base; end
  end
end

context 'conditional' do
  unless skip_tests
    TIMEOUT = 30
  end
end

# Constant inside case/when inside a block
describe 'config' do
  case mode
  when :production
    SETTING = true
  end
end

# Constant inside case/in (pattern matching) inside a block
describe 'config' do
  case value
  in Integer
    RESULT = value
  end
end

# Constant inside begin/rescue inside a block
describe 'retry' do
  begin
    LIMIT = calculate
  rescue StandardError
    nil
  end
end

# Constant inside a while loop inside a block
task :process do
  while items.any?
    BATCH_SIZE = 100
  end
end

# Constant inside a for loop inside a block
task :iterate do
  for item in items
    MARKER = true
  end
end

