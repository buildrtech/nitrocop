x = <<~SQL
  SELECT 1
SQL
y = <<~HTML
  <div>
HTML
z = <<~TEXT
  hello
TEXT
# Delimiters with non-word chars but containing word chars are meaningful
a = <<~'MY.SQL'
  SELECT 1
MY.SQL
b = <<-'END-BLOCK'
  content
END-BLOCK
c = <<~'my_template.html'
  content
my_template.html
# Backtick heredocs with meaningful delimiters
d = <<~`SHELL`
  echo hello
SHELL
# Empty heredocs with meaningful delimiters
e = <<~SQL
SQL
f = <<~HTML
HTML
