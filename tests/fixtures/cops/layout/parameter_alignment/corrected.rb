def foo(bar,
        baz)
  123
end

def method_a(x,
             y)
  x + y
end

def method_b(a,
             b)
  a + b
end

# Misaligned block parameter
def bidi_streamer(method, requests, marshal, unmarshal,
                  deadline: nil,
                  return_op: false,
                  parent: nil,
                  credentials: nil,
                  metadata: {},
                  &blk)
  blk.call
end

# Misaligned block parameter - simple case
def process(x,
            y,
            &block)
  block.call(x, y)
end

# Misaligned block parameter - another case
def handle(a,
           b,
           &blk)
  blk.call
end
