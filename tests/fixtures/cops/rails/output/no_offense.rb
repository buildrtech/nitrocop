$stdout.puts "hello"
io.print "hello"
Rails.logger.info "hello"
Logger.new(STDOUT).info("msg")
STDERR.puts "error"

# Block form (e.g. Phlex/Markaby HTML builders: p renders <p> tag)
p { "paragraph text" }
p do
  plain "some text"
end
p(class: "intro") { "hello" }

# Block pass argument
p(&:to_s)
puts(&block)

# Hash argument
print(flush: true)
