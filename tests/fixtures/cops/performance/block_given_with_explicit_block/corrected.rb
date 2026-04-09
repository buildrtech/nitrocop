def foo(&block)
  if block
    block.call
  end
end
def method(x, &block)
  do_something if block
end
def self.method(x, &block)
  do_something if block
end
def method(x, &myblock)
  do_something if myblock
end

# FN fix: block_given? inside negation (parsed as CallNode :! with receiver)
def negated_check(&block)
  raise "no block" unless block
end

def bang_negated(&block)
  return if !block
  block.call
end

# FN fix: block_given? as method argument
def as_argument(&block)
  log(block)
end

# FN fix: block_given? inside ternary
def in_ternary(&block)
  block ? block.call : default
end

# FN fix: qualified ::Kernel.block_given?
def with_qualified_kernel(&block)
  if block
    block.call
  end
end

def with_kernel_receiver(&block)
  do_something if block
end

# FN fix: block_given? as keyword argument value
def as_keyword_arg(&block)
  render(timing: block)
end

# FN fix: block_given? as default value for keyword parameter in method signature
def open(text, timing: block, &block)
  do_something
end

# FN fix: block_given? as default value for optional positional parameter
def process(flag = block, &block)
  do_something
end
