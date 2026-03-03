# nitrocop-filename: example.gemspec
# nitrocop-expect: 3:33 Gemspec/RequiredRubyVersion: `required_ruby_version` and `TargetRubyVersion` (3.4, which may be specified in .rubocop.yml) should be equal.
# nitrocop-expect: 5:33 Gemspec/RequiredRubyVersion: `required_ruby_version` and `TargetRubyVersion` (3.4, which may be specified in .rubocop.yml) should be equal.
Gem::Specification.new do |spec|
  if RUBY_PLATFORM =~ /aix/
    spec.required_ruby_version = ">= 3.0.3"
  else
    spec.required_ruby_version = ">= 3.1.0"
  end
  spec.name = 'example'
  spec.version = '1.0'
  spec.summary = 'A gem with conditional version'
end
