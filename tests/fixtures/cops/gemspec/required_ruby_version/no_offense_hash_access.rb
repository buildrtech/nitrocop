# nitrocop-filename: example.gemspec
Gem::Specification.new do |gem|
  gem.name = 'example'
  gem.version = '1.0'
  gem.required_ruby_version = gemspec['required_ruby_version']
  gem.summary = 'A gem with hash access version'
end
