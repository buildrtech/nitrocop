def +(other)
end
def -(other)
end
def ==(other)
end
def <=>(other)
end
def [](index)
end

# << is excluded from this cop
def <<(callable)
end

# Singleton methods are not checked
def ANY.==(_)
  true
end

# _other is accepted
def +(_other)
end

# Multiple parameters — not a binary operator signature
def *(a, b); end
def eql?(a, b); a == b; end
def equal?(node1, node2); node1 == node2; end

# Required param + block arg — more than one arg child
def ==(other_val, &block); other_val == self; end
