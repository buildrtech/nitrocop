source 'https://rubygems.org'
gem 'rubocop'
gem 'flog'
gem 'rails'
gem 'puma'
gem 'nokogiri'

if ENV['RAILS_VERSION'] == '5.2'
  gem 'sqlite3', '< 1.5', require: false
elsif ENV['RAILS_VERSION'] == '6.0'
  gem 'sqlite3', '1.5.1'
else
  gem 'sqlite3', '< 2', require: false
end

case
when ENV['RUBOCOP_VERSION'] == 'master'
  gem 'reek', git: 'https://github.com/troessner/reek.git'
else
  gem 'reek', '~> 6.0'
end
