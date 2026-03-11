class Foo
  def bar
    1
  end

  def baz
    2
  end

  # comment for qux
  def qux
    3
  end
end

# do..end block before definition — no blank line required
class Bar
  items.each do |item|
    process(item)
  end
  def foo
    1
  end

  def bar
    2
  end
end

# if..end before definition — no blank line required
class Baz
  if condition
    setup
  end
  def foo
    1
  end

  def bar
    2
  end
end

# begin..end before definition — no blank line required
class Qux
  begin
    setup
  end
  def foo
    1
  end
end

# Two defs separated by blank line + comments — no offense
class Quux
  def alpha
    1
  end

  # comment about bravo
  def bravo
    2
  end

  # first comment
  # second comment
  def charlie
    3
  end
end

# Adjacent single-line defs are allowed (AllowAdjacentOneLineDefs: true)
class Corge
  def alpha; 1 end
  def bravo; 2 end
  def charlie; 3 end
end

# Single-line def after multi-line def with blank line — no offense
class Grault
  def foo
    1
  end

  def bar; 2 end
end

# Multiple blank line groups (blanks on both sides of comments) — skipped
class Garply
  def alpha
    1
  end

  ### Section Header

  def bravo
    2
  end
end

# Multiple interleaved blank + comment groups — skipped
class Waldo
  def destroy
    1
  end

  # POST /auto_complete
  #-----
  # Handled by ApplicationController

  # PUT /suspend
  # PUT /suspend.xml
  #-----
  def suspend
    2
  end
end

# Endless methods with blank line between them
def compute() = x + y

def process() = z * w

# Adjacent endless methods (AllowAdjacentOneLineDefs: true by default)
def first_val() = 1
def second_val() = 2

# Endless method after regular method with blank line
class Fred
  def foo
    x
  end

  def bar() = y
end

# Defs inside conditionals (not siblings, so no offense)
if condition
  def foo
    true
  end
else
  def foo
    false
  end
end

# Nested def inside block (not siblings)
def build_model(*attrs)
  Class.new do
    def initialize(a)
    end
  end
end

# Single def after code line (only one def, not between defs)
x = 0
def calculate
end

# Single def after code and a comment
x = 0
# setup
def calculate
end

# Def after access modifier (only one candidate in pair)
class Service
  def alpha
    1
  end

  private

  def bravo
    2
  end
end

# First def in a class right after class keyword
class Simple
  def only_method
    1
  end
end

# First def in a module after public
module Helpers
  public
  #
  def escape(s)
  end
end
