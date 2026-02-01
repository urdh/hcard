# frozen_string_literal: true

source 'https://rubygems.org'
ruby '~> 3.3.0'

gem 'goodreads', '~> 0.8'
gem 'lastfm', '~> 1.27.x'
gem 'octokit', '~> 4.0'
gem 'webrick', '>= 0'

group :development, :test do
  gem 'dotenv', '~> 2.7'
  gem 'rspec', '~> 3.10'

  # Used by validate.rb
  gem 'colorize', '~> 0.8.1'
  gem 'html5_validator', '~> 1.0'
  gem 'html-proofer', '~> 5.0'
  gem 'nokogiri', '~> 1.18'
  gem 'open_uri_redirections', '~> 0.2.1'
  gem 'w3c_validators', '~> 1.3'
end

group :development, :lint do
  gem 'rubocop', '~> 1.7'
  gem 'rubocop-rspec', '~> 2.1'
end
