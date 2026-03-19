dest = []
src.each { |x| dest << x * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
result = []
items.each { |item| result << item.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
output = []
list.each { |e| output << transform(e) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
values = Array.new
src.each { |e| values << e.to_s }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
data = Array[]
src.each { |e| data << e * 2 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
# [].tap pattern with each and <<
[].tap { |res| src.each { |v| res << v } }
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
# [].tap with do...end block and each inside
[].tap do |files|
  directory.files.each { |file| files << file }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
end
# [].tap with push instead of <<
[].tap { |values| keys.each { |key| values << items[key] } }
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MapIntoArray: Use `map` instead of `each` to map elements into an array.
