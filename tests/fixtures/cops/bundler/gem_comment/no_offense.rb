source 'https://rubygems.org'

# Style-guide enforcer.
gem 'rubocop'

gem 'rails' # Web framework
# HTTP client
gem 'faraday'

=begin
gem 'legacy'
=end

# Non-literal gem names should not be flagged
gem db_gem, get_env("DB_GEM_VERSION")
gem plugin_name, :git => "https://example.com"
gem "social_stream-#{ g }"
gem ENV.fetch('MODEL_PARSER', nil)
gem tty_gem["name"], tty_gem["version"]

# Comment on a non-modifier if gem is fine
if ENV['USE_IT']
  # Required for special feature
  gem 'special_gem'
end

gem 'inline_comment_gem' if ENV['USE_IT'] # Needed for compatibility

# Multi-line gem with comment on continuation line should not be flagged
gem 'stream_rails', github: 'GetStream/stream-rails',
  branch: 'feature/subreference-enrichment' # Feed Enrichment

gem 'stripe_mock', github: 'stripe-ruby-mock/stripe-ruby-mock',
    require: 'stripe_mock' # Mock Stripe API
