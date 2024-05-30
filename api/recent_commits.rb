# frozen_string_literal: true

require 'octokit'
require 'json'

module RecentCommits
  GITHUB_USER = 'urdh'

  Handler = proc do |_req, res|
    begin
      client = Octokit::Client.new
      events = client.user_public_events(GITHUB_USER)
    rescue Octokit::Error => e
      res.status = e.response_status
      res.body = e.body
    else
      commits = events.select { |evt| evt[:type] == 'PushEvent' }.flat_map do |event|
        event.payload.commits.reverse_each.map do |commit|
          {
            sha: commit.sha,
            url: "https://github.com/#{event.repo.name}/commit/#{commit.sha}",
            message: commit.message.split("\n").first,
            repo: event.repo.name,
            date: event.created_at
          }
        end
      end
      res.status = 200
      res.body = commits.to_json
    end

    res['Content-Type'] = 'application/json; charset=utf-8'
    res['Cache-Control'] = 's-maxage=300, stale-while-revalidate'
  end
end

Handler = RecentCommits::Handler
