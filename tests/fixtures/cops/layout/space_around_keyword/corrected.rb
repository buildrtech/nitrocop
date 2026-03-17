if (x)
  y
end
unless (x)
  y
end
while (x)
  y
end
x = IO.read(__FILE__) rescue nil
x = 1 and 2
x = 1 or 2
x = a.to_s; y = b.to_s; z = c if (true)
case (ENV.fetch("DIST"))
when "redhat"
  puts "ok"
end
if true
  1
elsif (options.fetch(:cacheable))
  nil
end
x = defined? SafeYAML
x = super !=true
f = "x"
f.chop! until f[-1] != "/"
def bar; return (1); end
[1].each { |x|-> do end.call }
x = a==[]?self[m.to_s]: super
