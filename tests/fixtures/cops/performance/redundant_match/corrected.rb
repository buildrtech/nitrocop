x =~ /pattern/
x.match('string')
str =~ /\d+/
result =~ /#{expected}/
if str =~ /#{pattern}/
  do_something
end
# match in if body where the if value is not used (not last statement)
def parse(p)
  if p.look(/</)
    p =~ /</
    val = p.match(/[^>\n]*/)
    p =~ />/
  end
  return val
end
# match in nested if body where outer if value is not used
def process(p)
  if p.look(/#/)
    p =~ /#/
    if p.look(/static-/)
      tag = true
      p =~ /static-/
    end
    tag
  end
  tag
end
# match used only for truthiness inside string interpolation (modifier if)
"text #{value if str =~ /pattern/}"
