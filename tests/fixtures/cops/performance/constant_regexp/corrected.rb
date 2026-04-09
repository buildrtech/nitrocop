str.match?(/\A#{CONST}/o)
str.match?(/\A#{CONST1}something#{CONST2}\z/o)
str.match?(/\A#{CONST1}something#{Regexp.escape(CONST2)}\z/o)
rule %r/#{MISC_KEYWORDS}/io, Keyword
text.split(/#{REGEX_RANGE}/o).map do |time|
rule %r/#{SPACE}/o, Text
