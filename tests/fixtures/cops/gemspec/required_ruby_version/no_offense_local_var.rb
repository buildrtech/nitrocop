# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  version = '>= 2.5.0'
  spec.required_ruby_version = version
  spec.summary = 'A gem with local variable version'
end
