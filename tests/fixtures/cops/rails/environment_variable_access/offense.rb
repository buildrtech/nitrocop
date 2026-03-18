ENV['SECRET_KEY']
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV["DATABASE_URL"]
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV.fetch('REDIS_URL')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
::ENV.fetch('API_KEY')
^^^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['FOO'] = 'bar'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
::ENV['QUX'] = 'val'
^^^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV.store('KEY', 'value')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV.delete('KEY')
^^^ Rails/EnvironmentVariableAccess: Do not read from `ENV` directly post initialization.
ENV['BUNDLE_GEMFILE'] ||= File.expand_path('../Gemfile', __dir__)
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['RAILS_ENV'] ||= 'test'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
::ENV['APP_ENV'] ||= 'development'
^^^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['COUNTER'] &&= 'updated'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
ENV['COUNT'] += '1'
^^^ Rails/EnvironmentVariableAccess: Do not write to `ENV` directly post initialization.
