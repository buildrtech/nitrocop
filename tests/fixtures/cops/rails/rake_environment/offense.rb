task :foo do
^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  puts "hello"
end

task :bar do
^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  User.all.each { |u| puts u.name }
end

task :cleanup do
^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  OldRecord.delete_all
end

task 'generate_report' do
^^^^^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  Report.generate
end

task('update_cache') { Cache.refresh }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.

task migrate: [] do
^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  ActiveRecord::Base.connection.migrate
end

task refresh: [] do
^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  Cache.clear
end

task name do
^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  puts "local variable task name"
end

task(a.to_sym) { puts "method call task name" }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.

task short_name do
^^^^^^^^^^^^^^^^^^ Rails/RakeEnvironment: Add `:environment` dependency to the rake task.
  run_command
end
