# frozen_string_literal: true

require_relative '../api/recent_tracks'
require 'webrick'
require 'json'

RSpec.describe 'recent_tracks' do
  context 'after a successful request' do
    req = WEBrick::HTTPRequest.new(WEBrick::Config::HTTP)
    res = WEBrick::HTTPResponse.new(WEBrick::Config::HTTP)
    RecentTracks::Handler.call(req, res)
    res.body = JSON.parse(res.body)

    it 'returns a response with status 200' do
      expect(res.status).to eq 200
    end

    it 'returns a response with content type application/json' do
      expect(res['Content-Type']).to start_with 'application/json'
    end

    it 'returns a non-empty list of track metadata objects' do
      expect(res.body).to be_an_instance_of Array
      expect(res.body).not_to be_empty
    end

    it 'has assigned some metadata to every track object' do
      expect(res.body).to all(have_key('url'))
      expect(res.body).to all(have_key('title'))
      expect(res.body).to all(have_key('artist'))
      expect(res.body).to all(have_key('date'))
    end
  end
end
