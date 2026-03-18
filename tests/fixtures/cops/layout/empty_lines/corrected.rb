x = 1

y = 2

z = 3

# Consecutive blank lines between code and trailing comments
a = 1

# trailing comment

b = 2

# another comment

# Consecutive blank lines in a comment-only file
# frozen_string_literal: true

# Another comment

# Consecutive blank lines inside =begin/=end with code after =end
=begin
docs

more docs
=end
c = 3

# Consecutive blank lines before __END__ marker
d = 4

__END__
data after end marker
