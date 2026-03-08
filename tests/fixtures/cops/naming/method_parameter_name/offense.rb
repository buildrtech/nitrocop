def foo(x)
        ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def bar(a, bb)
        ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
           ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def baz(xy)
        ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def with_rest(*ab)
              ^^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def with_kwrest(**kw)
                ^^^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def with_block(&cb)
               ^^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def camel(fooBar)
          ^^^^^^ Naming/MethodParameterName: Only use lowercase characters for method parameter.
end
def camel_kw(number_One:)
             ^^^^^^^^^^ Naming/MethodParameterName: Only use lowercase characters for method parameter.
end
def camel_opt(varTwo = 1)
              ^^^^^^ Naming/MethodParameterName: Only use lowercase characters for method parameter.
end
def underscore_short(_a, _b)
                     ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
                         ^^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
def underscore_upper(_fooBar)
                     ^^^^^^^ Naming/MethodParameterName: Only use lowercase characters for method parameter.
end
def post_splat(*args, a, b)
                      ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
                         ^ Naming/MethodParameterName: Method parameter must be at least 3 characters long.
end
