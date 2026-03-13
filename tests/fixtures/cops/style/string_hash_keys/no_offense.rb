{ foo: 1 }
{ bar: 2, baz: 3 }
{ :foo => 1 }
x = { key: 'value' }
y = {}
z = { 1 => 'a' }

# IO.popen with env hash (string keys for env vars are exempted)
IO.popen({"RUBYOPT" => '-w'}, 'ruby', 'foo.rb')
IO.popen({"FOO" => "bar"}, 'cmd') do |io|
  io.read
end

# Open3 methods with env hash
Open3.capture3({"RUBYOPT" => '-w'}, 'ruby', 'foo.rb')
Open3.capture2({"PATH" => '/usr/bin'}, 'ls')
Open3.popen3({"HOME" => '/tmp'}, 'bash')

# Open3.pipeline with env hash inside array
Open3.pipeline([{"RUBYOPT" => '-w'}, 'ruby', 'foo.rb'], ['wc', '-l'])

# spawn/system with env hash
spawn({"FOO" => "bar"}, "cmd")
system({"FOO" => "bar"}, "cmd")
Kernel.spawn({"FOO" => "bar"}, "cmd")
Kernel.system({"FOO" => "bar"}, "cmd")

# gsub/gsub! with string replacement hash
"hello".gsub(/pattern/, "old" => "new")
"hello".gsub!(/pattern/, "old" => "new")
