[1, 2, 3].size
{a: 1}.size
[].size
(1..3).to_a.size
[[:foo, :bar], [1, 2]].to_h.size
Array[*1..5].size
Array(1..5).size
Hash[*('a'..'z')].size
Hash(key: :value).size
categories.to_a.size
post.comments.to_a.size
[1, 2, 3]&.size
(1..3)&.to_a&.size
[[:foo, :bar], [1, 2]]&.to_h&.size
# Multi-statement block — .count is NOT the direct body, still flagged
items.each { puts "hi"; [1, 2].size }
# .to_a.count nested inside hash inside array inside single-statement block
data.map { |r| [r[:id], { 'count' => r['items'].to_a.size }] }
# Chained .count inside single-statement block — .count is NOT the sole body
it "counts" do [:a, :b, :c].size.should == 3 end
