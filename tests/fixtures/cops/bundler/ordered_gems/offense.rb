gem 'rubocop'
gem 'rspec'
^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `rspec` should appear before `rubocop`.

gem 'zoo'
gem 'alpha'
^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `alpha` should appear before `zoo`.

gem 'puma'
gem 'nokogiri'
^^^^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `nokogiri` should appear before `puma`.

gem 'rubocop',
    '0.1.1'
gem 'rspec'
^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `rspec` should appear before `rubocop`.

platforms :jruby do
  gem "activerecord-jdbcsqlite3-adapter",
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcsqlite3-adapter.gemspec"
  gem "activerecord-jdbcpostgresql-adapter",
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `activerecord-jdbcpostgresql-adapter` should appear before `activerecord-jdbcsqlite3-adapter`.
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcpostgresql-adapter.gemspec"
  gem "activerecord-jdbcmysql-adapter",
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `activerecord-jdbcmysql-adapter` should appear before `activerecord-jdbcpostgresql-adapter`.
    git: "https://github.com/jruby/activerecord-jdbc-adapter",
    glob: "activerecord-jdbcmysql-adapter.gemspec"
end

group :development do
  if !ENV['WARBLER_SRC']; gem 'warbler' else gem 'warbler', path: '../../warbler' end
  if !ENV['RACK_SRC']; gem 'jruby-rack' else gem 'jruby-rack', path: '../../target' end
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `jruby-rack` should appear before `warbler`.
  gem 'jruby-jars', '~> 9.4.0'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Bundler/OrderedGems: Gems should be sorted in an alphabetical order within their section of the Gemfile. Gem `jruby-jars` should appear before `jruby-rack`.
end
