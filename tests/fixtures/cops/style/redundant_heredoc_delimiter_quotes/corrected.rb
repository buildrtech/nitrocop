do_something(<<~EOS)
  no string interpolation style text
EOS

do_something(<<-EOS)
  plain text here
EOS

do_something(<<EOS)
  just plain text
EOS

process(<<~END, option: { setting: '\u0031-\u0039' }, verbose: true)
  line one
  line two
END

run_code(<<~RUBY).should == "result\n"
  puts "hello".encoding
RUBY
