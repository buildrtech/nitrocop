IO.foreach('file') { |l| puts l }
File.foreach('file') { |l| puts l }
IO.readlines('file')
IO.readlines('file').size
File.readlines('testfile').not_enumerable_method
file.readlines.not_enumerable_method
::IO.foreach('file') { |l| puts l }
