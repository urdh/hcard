# frozen_string_literal: true

require 'goodreads'
require 'json'

module CurrentlyReading
  GOODREADS_USER = '27549920'

  Handler = proc do |_req, res|
    begin
      client = Goodreads.new(api_key: ENV.fetch('GOODREADS_API_KEY'),
                             api_secret: ENV.fetch('GOODREADS_SECRET'))
      updates = client.user(GOODREADS_USER).updates
    rescue Goodreads::Error => e
      res.status = 500
      res.body = { error: e.message }.to_json
    else
      books =  updates.select { |upd| upd.type == 'readstatus' }.map(&:object).map(&:read_status)
                      .select { |obj| obj.status == 'currently-reading' }.map(&:review)
                      .map do |review|
        {
          title: review.book.title,
          authors: ([] << review.book.author).flatten.map(&:name),
          url: "https://www.goodreads.com/book/show/#{review.book.id}",
          date: review.created_at
        }
      end
      res.status = 200
      res.body = books.to_json
    end

    res['Content-Type'] = 'application/json; charset=utf-8'
    res['Cache-Control'] = 's-maxage=86400, stale-while-revalidate'
  end
end

Handler = CurrentlyReading::Handler
