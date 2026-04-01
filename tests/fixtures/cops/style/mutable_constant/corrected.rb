CONST = [].freeze

CONST2 = {}.freeze

CONST3 = "hello".freeze

# ||= assignment is also flagged
CONST4 ||= [1, 2, 3].freeze

CONST5 ||= { a: 1, b: 2 }.freeze

CONST6 ||= 'str'.freeze

# %w and %i array literals
CONST7 = %w[a b c].freeze

CONST8 = %i[a b c].freeze

CONST9 = %w(foo bar).freeze

# Heredoc is mutable
CONST10 = <<~HERE
  some text
HERE

CONST11 = <<~RUBY
  code here
RUBY

# Module::CONST ||= value
Mod::CONST12 ||= [1].freeze

# Backtick (xstring) literals are mutable
CONST13 = `uname`.freeze

CONST14 = `echo hello`.freeze
