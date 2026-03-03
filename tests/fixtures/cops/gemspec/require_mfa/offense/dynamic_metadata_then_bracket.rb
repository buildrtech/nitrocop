# nitrocop-filename: example.gemspec
# nitrocop-expect: 1:0 Gemspec/RequireMFA: `metadata['rubygems_mfa_required']` must be set to `'true'`.
Gem::Specification.new do |gem|
  gem.name = 'example'
  gem.metadata = config['metadata'] if config['metadata']
  gem.metadata['rubygems_mfa_required'] = 'true'
end
