# nitrocop-filename: example.gemspec
# nitrocop-expect: 1:0 Gemspec/RequireMFA: `metadata['rubygems_mfa_required']` must be set to `'true'`.
Gem::Specification.new do |s|
  s.name = "example"
  s.version = "1.0"
  s.summary = "A gem with dynamic MFA value"
  s.authors = ["Author"]
  s.metadata["rubygems_mfa_required"] = false.to_s
end
