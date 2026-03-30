def foo
  super(a, b)
end

def bar
  super(x)
end

def baz
  super(1, 2, 3)
end

def qux(&block)
  super(&block)
end
