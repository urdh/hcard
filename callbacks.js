const Promise = require('bluebird');
const { Octokit } = require('@octokit/rest');
const GoodreadsApi = require('goodreads-api-node');
const LastfmApi = require('lastfmapi');
const Request = require('request-promise');

let Callbacks = function () { };

Callbacks.prototype.getRecentTracks = function (options) {
  'use strict';
  const lastfm = new LastfmApi({
    api_key: options.key,
    secret: options.secret
  });
  const apiGetRecentTracks = Promise.promisify(lastfm.user.getRecentTracks, { context: lastfm.user });
  return apiGetRecentTracks({ user: options.user }).then(function (result) {
    return [].concat.apply([], result.track.map(function (item) {
      const date = item.date || { 'uts': Date.now() / 1000 };
      return {
        'artist': item.artist['#text'],
        'title': item.name,
        'url': item.url,
        'date': new Date(date.uts * 1000).toISOString()
      };
    }));
  }).catch(function (err) {
    return { 'error': err };
  });
};

Callbacks.prototype.getCurrentBook = function (options) {
  'use strict';
  const goodreads = new GoodreadsApi({
    key: options.key,
    secret: options.secret
  });
  return goodreads.getUserInfo(options.user).then(function (result) {
    return [].concat.apply([], result.updates.update.filter(function (item) {
      return item.type == 'readstatus' && item.object.read_status.status == 'currently-reading';
    }).map(function (item) {
      var review = item.object.read_status.review;
      var authors = [].concat.apply(review.book.author).map(function (author) {
        return author.name;
      });
      return [{
        'title': review.book.title,
        'authors': authors,
        'url': 'http://www.goodreads.com/book/show/' + review.book.id._, // TODO
        'date': review.created_at._
      }];
    }));
  }).catch(function (err) {
    return { 'error': err };
  });
};

Callbacks.prototype.getGithubCommits = function (options) {
  'use strict';
  const github = new Octokit({});
  return github.activity.listEventsForUser({ username: options.user }).then(function (result) {
    return [].concat.apply([], result.data.filter(function (item) {
      return item.type == 'PushEvent';
    }).map(function (item) {
      return item.payload.commits.reverse().map(function (subitem) {
        return {
          'sha': subitem.sha,
          'url': 'http://github.com/' + item.repo.name + '/commit/' + subitem.sha,
          'message': subitem.message.split('\n')[0],
          'repo': item.repo.name,
          'date': item.created_at
        };
      });
    }));
  }).catch(function (err) {
    return { 'error': err };
  });
};

Callbacks.prototype.getPhotos = function (_) {
  'use strict';
  const options = {
    uri: 'https://photography.sigurdhsson.org/photos.json',
    json: true
  };
  return Request(options).then(function (result) {
    return result.map(function (item) {
      item.url = item.url.replace(/index.html?/g,'');
      return item;
    });
  }).catch(function (err) {
    return { 'error': err };
  });
};

module.exports = new Callbacks();
