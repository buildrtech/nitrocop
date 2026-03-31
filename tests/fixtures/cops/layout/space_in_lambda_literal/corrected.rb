a = ->(x, y) { x + y }
b = ->(x) { x * 2 }
c = ->(a, b, c) { a + b + c }
d = ->x { x + 1 }
e = ->x, y { x + y }
f = ->x { ->y { x + y } }
g = ->*args { args }
