# nitrocop-filename: example.gemspec
# nitrocop-expect: 1:0 Gemspec/RequireMFA: `metadata['rubygems_mfa_required']` must be set to `'true'`.
Gem::Specification.new do |s|
  s.name = 'example'
  s.metadata = {
    'homepage_uri' => 'https://example.com',
  }
  s.metadata['rubygems_mfa_required'] = 'true'
end
