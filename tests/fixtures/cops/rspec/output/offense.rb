p "edmond dantes"
^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

puts "sinbad"
^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

print "abbe busoni"
^^^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

pp "monte cristo"
^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

$stdout.write "lord wilmore"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

STDERR.write "bertuccio"
^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

# Output calls inside blocks that are arguments to other calls
system(*%w[make].tap { |cmd| puts cmd.inspect })
                             ^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.

items.each { |x| p x }
                 ^^^ RSpec/Output: Do not write to stdout in specs.

# Output calls inside lambdas/procs
-> { 1.upto(10) { |x| p x } }.call
                      ^^^ RSpec/Output: Do not write to stdout in specs.

x = -> do
  print "hello"
  ^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
end

# Output calls inside Proc.new blocks chained with .call
Proc.new do
  print "inside proc"
  ^^^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
end.call

# Output calls inside lambda used as receiver of .should
-> { p(obj) }.should output("test\n")
     ^^^^^^ RSpec/Output: Do not write to stdout in specs.

# Output calls inside method definitions
def self.compile(path)
  puts "compiling"
  ^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
end

# Output calls deeply nested inside blocks within method definitions
FileUtils.cd(dir) do
  unless system(*args.tap { |cmd| puts cmd.inspect })
                                  ^^^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
    puts "error"
    ^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
  end
end

# Output calls inside array arguments (parent is ArrayNode, not CallNode)
result = [p, 42]
          ^ RSpec/Output: Do not write to stdout in specs.

expect(e.relatives).to match_array [p, c]
                                    ^ RSpec/Output: Do not write to stdout in specs.

# Output calls inside hash values (parent is AssocNode, not CallNode)
{ key: puts("hello") }
       ^^^^^^^^^^^^^^ RSpec/Output: Do not write to stdout in specs.
