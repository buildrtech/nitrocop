"result is #{x == 'foo'}"

"hello #{hash['key']}"

"test #{y.gsub('a', 'b')}"

"escape #{visit '\\'}"

"split #{value.split('\\').last}"

# Double-quoted string inside interpolation nested within a backtick xstr
`cmd #{items.map { |item| "#{item['id']}" }.join(" ")}`
