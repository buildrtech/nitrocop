source 'https://rubygems.org'

gem 'rubocop'
^ Bundler/GemComment: Missing gem description comment.

gem 'rails'
^ Bundler/GemComment: Missing gem description comment.

gem 'puma'
^ Bundler/GemComment: Missing gem description comment.

# frozen_string_literal: true
gem 'foo'
^ Bundler/GemComment: Missing gem description comment.

# This comment is about the condition, not the gem
gem 'conditional_gem' if ENV['USE_IT']
^ Bundler/GemComment: Missing gem description comment.

# Version 2.1 introduces breaking change baz
gem "fog-google", "~> 1.13.0" if RUBY_VERSION.to_f < 2.7
^ Bundler/GemComment: Missing gem description comment.

# Some browsers have problems with WEBrick
gem 'puma_alt' unless RUBY_ENGINE == 'truffleruby'
^ Bundler/GemComment: Missing gem description comment.

# This comment is about the condition, not the gem; %(#) is NOT a real comment
gem 'rouge', (ENV.fetch 'ROUGE_VERSION', %(~> #{RUBY_ENGINE == 'jruby' ? '3' : '4'}.0)), require: false unless ENV['ROUGE_VERSION'] == 'false'
^ Bundler/GemComment: Missing gem description comment.
