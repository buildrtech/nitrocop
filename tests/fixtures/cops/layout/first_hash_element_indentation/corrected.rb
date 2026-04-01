x = {
  a: 1,
  b: 2
}
y = {
  c: 3,
  d: 4
}
z = {
  e: 5,
  f: 6
}

buffer << {
}

value = {
  a: 1
}

wrap({
       a: 1
     })

func(x: {
       a: 1,
       b: 2
     },
     y: {
       c: 1,
       d: 2
     })

# Hash inside double-splat (**{}) in method call — first element wrong indent
# paren at col 9, base = 9+1=10, expected = 10+2=12, actual = 4
translate('msg', **{
            :key => 'val',
    :cls => klass.to_s
          })

# Hash inside double-splat — right brace wrong indent
# paren at col 9, expected closing = 10
translate('msg', **{
            :key => 'val',
                   :cls => klass.to_s
          })

# Hash inside local var assignment in method args
# paren at col 21, base = 21+1=22, expected = 22+2=24, actual = 4
migration.proper_name(table, options = {
                        prefix: Base.prefix,
    suffix: Base.suffix
                      })

# Hash inside ternary in method call args
# paren at col 20, expected closing = 21
Autoprefixer.install(self, safe ? config : {
                     })
