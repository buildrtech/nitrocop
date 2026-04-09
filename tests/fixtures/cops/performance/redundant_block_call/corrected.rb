def method(&block)
  yield
end

def method2(&block)
  yield(1, 2)
end

def method3(&block)
  yield(arg)
end

def filter!(&block)
  items.each do |item|
    yield(item) if block_given?
  end
end

# block.call inside inner block that shadows &block param (multi-statement method)
def process_callbacks(name, &block)
  logger.debug "running callbacks"
  Array(callbacks[name]).each do |block|
    yield(args)
  end
end

# block.call inside inline block that shadows &block (multi-statement method)
def run_hooks(&block)
  setup_state
  @hooks.each { |block| yield(config) }
