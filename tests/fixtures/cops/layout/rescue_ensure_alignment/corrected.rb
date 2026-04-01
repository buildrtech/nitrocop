begin
  foo
rescue => e
  bar
end

begin
  foo
ensure
  bar
end

begin
  baz
rescue => e
  qux
ensure
  grault
end

# Misaligned rescue inside def
def fetch(key)
  @store[key]
rescue
    raise RetrievalError
end

# Misaligned ensure inside def
def process
  do_work
ensure
    cleanup
end

# Misaligned rescue + ensure inside def
def transform
  compute
rescue StandardError => e
    handle(e)
ensure
    finalize
end

# Misaligned rescue inside class
class Widget
  do_something
rescue
    handle_error
end

# Misaligned ensure inside module
module Helpers
  initialize_stuff
ensure
    finalize
end

# Misaligned rescue inside do-end block
items.each do |item|
  item.process
rescue StandardError
    next
end

# Misaligned ensure inside do-end block
records.map do |rec|
  rec.save
ensure
    rec.unlock
end

# Rescue chain: second rescue misaligned in begin
begin
  risky_call
rescue Timeout::Error
  retry
rescue
end
