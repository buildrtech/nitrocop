x < y && y < z
10 <= x && x <= 20
a > b
x < y
a >= b && b <= c
x == y
a != b
x < y || y > z
min <= value && value <= max

# Set operations as center value should not be flagged
x >= y & x < z
x >= y | x < z
x >= y ^ x < z
