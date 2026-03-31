"Hello#{name}"

"foo" + "bar"

"prefix_#{value.to_s}"

# Chain: one offense for the whole chain (at innermost string-concat node)
user.name + ' <' + user.email + '>'

# Chain where only the RHS is a string — fires once at topmost
a + b + 'c'

# Chain where only the LHS is a string — fires once at innermost
"a#{b}" + c

# Mixed chain: string deep in receiver, string at end
a + 'b' + c + 'd'

# Single non-literal + string (aggressive mode)
Pathname.new('/') + 'test'

# Heredoc with single-line content (str in Parser) — flagged
code = <<EOM + extra_code
content
EOM

# Single-line string with escape \n (not multi-line source) — flagged
"hello\nworld#{name}"
