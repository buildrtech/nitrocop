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

# Modifier if inside case/when should not create a separate conditional root
case rails_version.segments[0]
when 6
  gem 'concurrent-ruby', '< 1.3.5'
when 7
  gem 'concurrent-ruby', '< 1.3.5' if rails_version.segments[1] < 2
end

# Gems in if/else where the else branch wraps in a single-statement group block
# RuboCop treats the gem as a direct child of the else branch (Parser gem quirk:
# single-statement block bodies expose their child as a direct child_node of the block)
if ENV["ALLOW_DEV_POPULATE"] == "1"
  gem "faker"
else
  group :development, :test do
    gem "faker"
  end
end

# Duplicated gem inside if...end (no else) — conditional exemption
if RUBY_VERSION >= "3.2.0"
  gem "minitest-mock"
  gem "async", "~>2.0"
  gem "minitest-mock"
end
