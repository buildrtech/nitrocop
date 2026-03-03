# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.add_dependency 'foo', '~> 1.0'
  spec.add_dependency 'bar', '>= 2.0'
  spec.add_development_dependency 'rspec', '~> 3.0'
  spec.add_dependency %q<os>, "~> 1.1", ">= 1.1.4"
  spec.add_dependency %q(parser), '~> 3.0'
  spec.add_dependency %q[json], '>= 2.0'
  spec.add_dependency %Q<rubocop-ast>, '~> 1.0'
  spec.add_runtime_dependency %q<rake>, '~> 13.0'
  spec.add_development_dependency %q<minitest>, '~> 5.0'
  spec.authors = ['Author']
end
