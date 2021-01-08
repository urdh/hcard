# frozen_string_literal: true

require_relative '../api/recent_commits'
require 'webrick'
require 'json'

RSpec.describe 'recent_commits' do
  context 'after a successful request' do
    req = WEBrick::HTTPRequest.new(WEBrick::Config::HTTP)
    res = WEBrick::HTTPResponse.new(WEBrick::Config::HTTP)
    RecentCommits::Handler.call(req, res)
    res.body = JSON.parse(res.body)

    it 'returns a response with status 200' do
      expect(res.status).to eq 200
    end

    it 'returns a response with content type application/json' do
      expect(res['Content-Type']).to start_with 'application/json'
    end

    it 'returns a non-empty list of commit metadata objects' do
      expect(res.body).to be_an_instance_of Array
      expect(res.body).not_to be_empty
    end

    it 'has assigned some metadata to every commit object' do
      expect(res.body).to all(have_key('sha'))
      expect(res.body).to all(have_key('url'))
      expect(res.body).to all(have_key('message'))
      expect(res.body).to all(have_key('repo'))
      expect(res.body).to all(have_key('date'))
    end
  end
end
