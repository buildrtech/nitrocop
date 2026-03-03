# nitrocop-filename: example.gemspec
# nitrocop-expect: 2:31 Gemspec/RequiredRubyVersion: `required_ruby_version` and `TargetRubyVersion` (3.4, which may be specified in .rubocop.yml) should be equal.
Gem::Specification.new do |spec|
  spec.required_ruby_version = '>= 3'
  spec.name = 'example'
  spec.version = '1.0'
  spec.summary = 'A gem with single digit version'
end
