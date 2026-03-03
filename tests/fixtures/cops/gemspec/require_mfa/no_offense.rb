# nitrocop-filename: example.gemspec
Gem::Specification.new do |spec|
  spec.name = 'example'
  spec.version = '1.0'
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.authors = ['Author']
  spec.summary = 'An example gem'
end

# Also detect hash-style metadata assignment
Gem::Specification.new do |s|
  s.name = 'example2'
  s.metadata = {
    'rubygems_mfa_required' => 'true'
  }
end

# Positional args form: Gem::Specification.new "name", version do |s|
# RuboCop's NodePattern does NOT match this form, so no offense.
Gem::Specification.new "example3", "1.0.0" do |gem|
  gem.authors = ["Author"]
  gem.summary = "A gem with positional args"
end

# Positional args with constant version
Gem::Specification.new "example4", Example::VERSION do |s|
  s.summary = "Another gem with positional args"
end
