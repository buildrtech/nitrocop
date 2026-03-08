def method(&block)
  block.call
  ^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end

def method2(&block)
  block.call(1, 2)
  ^^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end

def method3(&block)
  block.call(arg)
  ^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end

def filter!(&block)
  items.each do |item|
    block.call(item) if block_given?
    ^^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
  end
end

# block.call inside inner block that shadows &block param (multi-statement method)
def process_callbacks(name, &block)
  logger.debug "running callbacks"
  Array(callbacks[name]).each do |block|
    block.call(args)
    ^^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
  end
end

# block.call inside inline block that shadows &block (multi-statement method)
def run_hooks(&block)
  setup_state
  @hooks.each { |block| block.call(config) }
                        ^^^^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
