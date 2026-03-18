x = 1


^ Layout/EmptyLines: Extra blank line detected.
y = 2


^ Layout/EmptyLines: Extra blank line detected.

^ Layout/EmptyLines: Extra blank line detected.
z = 3

# Consecutive blank lines between code and trailing comments
a = 1


^ Layout/EmptyLines: Extra blank line detected.
# trailing comment

b = 2


^ Layout/EmptyLines: Extra blank line detected.
# another comment

# Consecutive blank lines in a comment-only file
# frozen_string_literal: true


^ Layout/EmptyLines: Extra blank line detected.
# Another comment

# Consecutive blank lines inside =begin/=end with code after =end
=begin
docs


^ Layout/EmptyLines: Extra blank line detected.
more docs
=end
c = 3

# Consecutive blank lines before __END__ marker
d = 4


^ Layout/EmptyLines: Extra blank line detected.
__END__
data after end marker
