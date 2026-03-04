x.match?(/pattern/)
x.match?(y)
x == y
x.include?("pattern")
x.start_with?("p")
# =~ not in a condition context — not flagged
result = x =~ /pattern/
x =~ /re/
arr.select { |s| s =~ /re/ }
def matches?(val)
  val =~ /re/
end
# =~ in condition but MatchData is used — not flagged
if x =~ /pattern/
  Regexp.last_match(1).downcase
end
if str =~ /(\d+)/
  puts $1
end
# =~ or !~ inside && chain — not flagged (RuboCop only checks top-level condition)
if foo && bar =~ /re/
  do_something
end
if request.path !~ /pattern/ && other
  redirect
end
# while/until conditions — not flagged (RuboCop only checks if/unless/case)
while str =~ /\d+/
  process
end
line = input.gets while line !~ /^>THREE/
raw_route_info.shift until raw_route_info[0] =~ /Destination/i
# =~ with named captures (creates local vars) — not flagged
if /alias_(?<alias_id>.*)/ =~ something
  do_something
end
# .match() without regexp/str/sym literal — not flagged
if CONST.match(var)
  do_something
end
# match without arguments — not flagged
code if match
# MatchData is used in same method scope — not flagged
def foo
  if x =~ /re/
    do_something($1)
  end
end
def bar
  return $~ if x =~ /re/
end
# MatchData used after guard return — not flagged
def check
  return if str =~ /pattern/
  do_something($1)
end
# $+ (last successful match) — not flagged
if config.verbose.to_s =~ /^-?(v+)$/
  result = $+
end
# =~ inside case/in pattern matching guard — not flagged
case val
in pattern if val =~ /regex/
  :ok
end
case val
in { name: name } if name =~ /human/i
  puts name
end
# MatchData ref in block within method — still suppresses
def with_block_ref
  bar do
    if x =~ /re/
      do_something
    end
  end
  puts $1
end
# Sequential modifier-if: each =~ has MatchData used in its own if-branch body
def sequential_with_matchdata
  raise "missing #{$1}" if out =~ /^ERROR.+Unknown '(.+)'/
  raise "no table #{$1}" if out =~ /^ERROR.+Table '.+\.(.+)'/
end
# =~ in modifier-if where later line's $1 belongs to later match
def later_match_uses_matchdata
  raise "not running" if out =~ /^ERROR.+Can't connect/
  raise "missing #{$1}" if out =~ /^ERROR.+Unknown '(.+)'/
end
# match() in modifier-if where later line's $1 belongs to later match
def match_method_with_later_matchdata
  ie_comment += "gte " if cond.match(/or_higher/)
  ie_comment += " #{$1}" if cond.match(/IE(\d)/)
end
# case/when where first when uses MatchData
case
when line =~ /^ORIGINAL ?([\w\s]+)$/
  name = $~[1].strip
when line =~ /^(.+)$/
  opts = eval("{ #{$~[1]} }")
end
