File.readlines('testfile').map { |l| l.size }
     ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.map` instead of `readlines.map`.
IO.readlines('testfile').map { |l| l.size }
   ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.map` instead of `readlines.map`.
IO.readlines('testfile').each { |l| puts l }
   ^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line` instead of `readlines.each`.
file.readlines(10).map { |l| l.size }
     ^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.map` instead of `readlines.map`.
file.readlines(10).each { |l| puts l }
     ^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line` instead of `readlines.each`.
child_pipe.readlines.first
           ^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.first` instead of `readlines.first`.
File.readlines(file).select { |e| e.strip.length > 0 }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.select` instead of `readlines.select`.
IO.readlines('file').reject { |l| l.empty? }
   ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.reject` instead of `readlines.reject`.
f.readlines.find { |l| l.start_with?('#') }
  ^^^^^^^^^^^^^^ Performance/IoReadlines: Use `each_line.find` instead of `readlines.find`.
