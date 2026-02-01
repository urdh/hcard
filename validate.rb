#!/usr/bin/env ruby
# frozen_string_literal: true

require 'rubygems'
require 'nokogiri'
require 'html5_validator'
require 'w3c_validators'
require 'open-uri'
require 'open_uri_redirections'
require 'html-proofer'
require 'colorize'

IGNORED_FILES = [].freeze

# Helper for reading a file.
def get_contents(file)
  if file.respond_to? :read
    file.reading
  else
    read_local_file(file)
  end
end

# HACK: w3c_validators doesn't provide a generic XML validator.
# We provide a replacement based on Nokogiri with compatible interface.
class XMLValidator < W3CValidators::Validator
  def validate_against_schema(document)
    schema_uri = document.xpath('*/@xsi:schemaLocation').to_s.split[1]
    schema = URI.parse(schema_uri).open(allow_redirections: :safe)
    Nokogiri::XML::Schema(schema.read).validate(document)
  end
  private :validate_against_schema

  def validate_file(file) # rubocop:disable Metrics/MethodLength
    src = get_contents(file)

    begin
      document = Nokogiri::XML(src)
      if document.xpath('*/@xsi:schemaLocation').empty?
        @results = W3CValidators::Results.new({ uri: nil, validity: true })
      else
        errors = validate_against_schema(document)
        @results = W3CValidators::Results.new({ uri: nil, validity: errors.empty? })
        errors.each { |msg| @results.add_error({ message: msg.to_s }) if msg.error? }
      end
    rescue StandardError
      @results = W3CValidators::Results.new({ uri: nil, validity: false })
      @results.add_error({ message: 'Nokogiri threw errors on input.' })
    end
    @results
  end
end

# HACK: W3CValidators::NuValidator seems broken and the gem looks unmaintained.
# We provide our own replacement based on the html5_validator gem, with compatible interface.
class HtmlValidator < W3CValidators::Validator
  def validate_file(file)
    src = get_contents(file)

    validator = Html5Validator::Validator.new
    validator.validate_text(src)
    @results = W3CValidators::Results.new({ uri: nil, validity: validator.valid? })
    validator.errors.each { |err| @results.add_error({ message: err['message'] }) }
    @results
  end
end

puts "Validating jekyll output in 'public/'..."
puts "\n"
failed = 0
passed = 0
skipped = 0

# Iterate over all the site files and validate them as appropriate.
Dir.glob('public/**/*') do |file|
  # Skip ignored files and all directories
  next if File.directory?(file)
  next if IGNORED_FILES.include? file

  # Since all validators have compatible interfaces, we create the appropriate instance...
  validator = case File.extname(file)
              when '.html'
                HtmlValidator.new
              when '.xml'
                if File.basename(file) == 'atom.xml'
                  W3CValidators::FeedValidator.new
                else
                  XMLValidator.new
                end
              when '.css'
                W3CValidators::CSSValidator.new
              else
                skipped += 1
                puts file.colorize(:light_black)
                next
              end

  # ... and then run the validation.
  if validator.validate_file(file).errors.empty?
    puts file.colorize(:green)
    passed += 1
  else
    puts file.colorize(:red)
    failed += 1
  end
end

puts "Running html-proofer in content in 'public/'..."
puts "\n"
htmlproofer = HTMLProofer.check_directory('./public', { ssl_verifyhost: 2,
                                                        only_4xx: true,
                                                        ignore_urls: [%r{www.reddit.com/user/urdh}],
                                                        parallel: { in_processes: 3 },
                                                        disable_external: ARGV.include?('--disable-external') }).run

puts "\n"
puts "#{passed} files pass validation, #{failed} files failed."
puts 'The html-proofer test failed!' unless htmlproofer

failed += 1 unless htmlproofer
exit failed
