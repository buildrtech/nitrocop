def foo(...)
  bar(...)
end

def baz(x, y)
  qux(x, y)
end

def test
  42
end

# Non-redundant names: *items and &handler are NOT in the default redundant lists
# So neither anonymous forwarding nor ... forwarding applies
def self.with(*items, &handler)
  new(*items).tap(&handler).to_element
end

# Non-redundant block and rest names — no forwarding suggestions
def process(*entries, &callback)
  entries.each(&callback)
end

# Both args referenced directly — no anonymous forwarding possible
def capture(*args, &block)
  args.each { |a| puts a }
  block.call
  run(*args, &block)
end

# No body — nothing to forward to
def empty(*args, &block)
end

# Multi-assignment reassigns the kwrest param — no anonymous forwarding
def where(attribute, type = nil, **options)
  attribute, type, options = normalize(attribute, type, **options)
  @records.select { |r| r.match?(attribute, type, **options) }
end

# ||= reassigns the block param — no anonymous block forwarding
def run(cmd, &block)
  block ||= default_handler
  execute(cmd, &block)
end

# kwrest used as a hash (not forwarding) — options[:key] reads it directly
def build(salt, **options)
  length = compute_length(*options[:cipher])
  Encryptor.new(**options)
end

# &&= reassigns the args param
def process(*args)
  args &&= args.compact
  handle(*args)
end

# Multi-assignment reassigns the block param
def task(name, &block)
  name, deps, block = *parse_deps(name, &block)
  define_task(name, *deps, &block)
end

