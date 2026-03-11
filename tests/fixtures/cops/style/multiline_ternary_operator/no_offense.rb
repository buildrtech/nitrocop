a = cond ? b : c

foo ? bar : baz

x ? 1 : 2

result = x > 0 ? 'positive' : 'non-positive'

do_something(arg.foo ? bar : baz)

options.merge(
  current_page > 1 ? {
    previous_page: {
      href: page_path(current_page - 1),
    },
  } : {},
)
