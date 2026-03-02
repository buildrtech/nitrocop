# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.summary = 'An example gem'
  spec.authors = ['Author']
  spec.files = Dir['lib/**/*']
  spec.homepage = 'https://example.com'
end

Gem::Specification.new do |spec|
  spec.name = 'example'
  s.date = '2024-01-01'
  s.test_files += Dir.glob('test/**/*')
end

metadata.date = '2024-01-01'

build_spec do |spec|
  spec.test_files = Dir.glob('test/**/*')
end
