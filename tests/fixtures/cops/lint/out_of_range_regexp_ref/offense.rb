puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (no regexp capture groups detected).
/(foo)(bar)/ =~ "foobar"
puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (2 regexp capture groups detected).
/(?<foo>FOO)(?<bar>BAR)/ =~ "FOOBAR"
puts $3
     ^^ Lint/OutOfRangeRegexpRef: $3 is out of range (2 regexp capture groups detected).
# Chained calls: gsub with captures in receiver, but =~ on RHS has fewer captures
if "foo bar".gsub(/(a)(b)/, "") =~ /foobar/
  p $1
    ^^ Lint/OutOfRangeRegexpRef: $1 is out of range (no regexp capture groups detected).
end
# After gsub on $N receiver, capture count is updated to gsub's regexp
"some : line " =~ / : (.+)/
$1.gsub(/\s{2}/, " ")
puts $1
     ^^ Lint/OutOfRangeRegexpRef: $1 is out of range (no regexp capture groups detected).
