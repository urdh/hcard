# frozen_string_literal: true

require 'lastfm'
require 'json'
require 'date'

class String # :nodoc:
  def as_uts
    DateTime.strptime(self, '%s')
  end
end

module RecentTracks
  LASTFM_USER = 'TinyGuy'

  Handler = proc do |_req, res|
    begin
      client = Lastfm.new(ENV.fetch('LASTFM_API_KEY'), ENV.fetch('LASTFM_SECRET'))
      tracks = client.user.get_recent_tracks(LASTFM_USER)
    rescue Lastfm::ApiError => e
      res.status = 500
      res.body = { error: e.message }.to_json
    else
      tracks.map! do |track|
        {
          artist: track.dig('artist', 'content'),
          title: track['name'],
          url: track['url'],
          date: track.dig('date', 'uts')&.as_uts || DateTime.now
        }
      end
      res.status = 200
      res.body = tracks.to_json
    end

    res['Content-Type'] = 'application/json; charset=utf-8'
    res['Cache-Control'] = 's-maxage=150, stale-while-revalidate'
  end
end

Handler = RecentTracks::Handler
