# frozen_string_literal: true

require_relative '../api/currently_reading'
require 'webrick'
require 'json'

RSpec.describe 'currently_reading' do
  context 'after a successful request' do
    req = WEBrick::HTTPRequest.new(WEBrick::Config::HTTP)
    res = WEBrick::HTTPResponse.new(WEBrick::Config::HTTP)
    CurrentlyReading::Handler.call(req, res)
    res.body = JSON.parse(res.body)

    it 'returns a response with status 200' do
      expect(res.status).to eq 200
    end

    it 'returns a response with content type application/json' do
      expect(res['Content-Type']).to start_with 'application/json'
    end

    it 'has assigned some metadata to every book object' do
      expect(res.body).to all(have_key('url'))
      expect(res.body).to all(have_key('title'))
      expect(res.body).to all(have_key('authors'))
      expect(res.body).to all(have_key('date'))
    end
  end
end
