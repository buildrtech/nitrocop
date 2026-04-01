class Foo
  def bar
    <<~SQL
      'Hi'
    SQL
  end
end

class Baz
  def qux
    <<~RUBY
      something
    RUBY
  end
end

def example
  <<-TEXT
    hello
  TEXT
end

# Heredoc in block body should still flag offense
get '/foo' do
    <<-EOHTML
    <html></html>
    EOHTML
end

# Heredoc as argument with wrong closing alignment (matches neither opening nor call)
include_examples :offense,
                 <<-HEREDOC
  bar
HEREDOC
