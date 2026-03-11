def my_method() = x
def my_method(a, b) = x
def regular_method
  x
end
def another_method
  x.foo
  x.bar
end

# Assignment methods (setter methods ending with =) are always skipped by RuboCop
def my_method=(value) = value.strip
def name=(value) = @name = value
def status=(val) = @status = val.to_s
                                .strip

# Shovel operator (<<) should NOT be confused with heredoc
def append(item) = @items << item

# Heredoc in endless method body should be skipped
def my_method = <<~HEREDOC
  hello
HEREDOC

# Heredoc in descendant of endless method body
def my_method = puts <<~HEREDOC
  hello
HEREDOC

# Singleton endless methods (`defs`) are skipped by RuboCop for this cop
def self.lookup(value) =
  case value
  in 1 | 2
    true
  else
    false
  end

def self.payload() =
  {
    a: 1,
    b: 2
  }
