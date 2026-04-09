File.readlines('testfile').map { |l| l.size }
IO.readlines('testfile').map { |l| l.size }
IO.readlines('testfile').each { |l| puts l }
file.each_line(10).map { |l| l.size }
file.each_line(10) { |l| puts l }
child_pipe.each_line.first
File.readlines(file).select { |e| e.strip.length > 0 }
IO.readlines('file').reject { |l| l.empty? }
f.each_line.find { |l| l.start_with?('#') }
