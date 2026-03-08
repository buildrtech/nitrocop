task foo: :environment do
  puts "hello"
end
task :bar => :environment do
  puts "world"
end
task :notices_delete, [:problem_id] => [:environment] do
  puts "delete"
end
task :baz, [:arg] => [:environment, :other] do
  puts "multi deps"
end
task :default do
  puts "default task"
end
task default: [:test] do
  puts "default with deps"
end
task setup: :database do
  puts "setup depends on database"
end
