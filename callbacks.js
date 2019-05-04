var Promise = require('bluebird');
var GitHubApi = require('@octokit/rest');
var GoodreadsApi = require('goodreads-api-node');
var LastfmApi = require('lastfmapi');

var Callbacks = function () { };

Callbacks.prototype.getRecentTracks = function (options) {
  'use strict';
  var lastfm = new LastfmApi({
    api_key: options.key,
    secret: options.secret
  });
  var apiGetRecentTracks = Promise.promisify(lastfm.user.getRecentTracks, { context: lastfm.user });
  return apiGetRecentTracks({ user: options.user }).then(function (result) {
    return [].concat.apply([], result.track.map(function (item) {
      var date = item.date || { 'uts': Date.now() / 1000 };
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
  var goodreads = new GoodreadsApi({
    key: options.key,
    secret: options.secret
  });
  return goodreads.getUserInfo(options.user).then(function (result) {
    var status = result.user_statuses.user_status;
    if (status === undefined) {
      return [];
    } else {
      var authors = [].concat.apply(status.book.authors.author).map(function (author) {
        return author.name;
      });
      return [{
        'title': status.book.title,
        'authors': authors,
        'url': 'http://www.goodreads.com/book/show/' + status.book.id._, // TODO
        'date': status.created_at._
      }];
    }
  }).catch(function (err) {
    return { 'error': err };
  });
};

Callbacks.prototype.getGithubCommits = function (options) {
  'use strict';
  var github = new GitHubApi({});
  var getEvents = Promise.promisify(github.activity.getEventsForUserPublic, { context: github.activity });
  return getEvents({ username: options.user }).then(function (result) {
    return [].concat.apply([], result.data.filter(function (item) {
      return item.type == 'PushEvent';
    }).map(function (item) {
      var repo = item.repo.name;
      return item.payload.commits.reverse().map(function (subitem) {
        return {
          'sha': subitem.sha,
          'url': 'http://github.com/' + repo + '/commit/' + subitem.sha,
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

Callbacks.prototype.get500pxPhotos = function (_) {
  'use strict';
  return [];
};

module.exports = new Callbacks();
