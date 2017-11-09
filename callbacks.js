var Promise = require('bluebird');
var GitHubApi = require('github');
var GoodreadsApi = require('goodreads');
var LastfmApi = require('lastfmapi');
var Api500px = require('500px');

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
  var goodreads = new GoodreadsApi.client({
    key: options.key,
    secret: options.secret
  });
  var getBooks = Promise.promisify(goodreads.getSingleShelf, { context: goodreads });
  // Shitty goodreads node module is shitty and doesn't "conform to node.js
  // convention of accepting a callback as last argument and calling that
  // callback with error as the first argument and success value on the
  // second argument", the callback only accepting one argument containing data.
  return getBooks({ userID: options.user, shelf: 'currently-reading' }).then(function () {
    // Data should be here, but it isn't.
    return { 'error': 'Goodreads API module has been fixed?' };
  }).catch(function (result) {
    if (!result || !result.GoodreadsResponse) {
      return { 'error': 'Bad response from Goodreads API' };
    }
    // Here's the data.
    return [].concat.apply([], result.GoodreadsResponse.books.map(function (item) {
      return item.book.map(function (subitem) {
        var authors = subitem.authors[0].author.map(function (subsubitem) {
          return subsubitem.name[0] +
            ((subsubitem.role[0] !== '') ? (' (' + subsubitem.role[0] + ')') : '');
        }).reduce(function (prev, curr, idx, arr) {
          if (arr.length <= 1) {
            return curr;
          }
          if (idx == arr.length - 1) {
            return prev + ' and ' + curr;
          }
          return prev + ', ' + curr;
        }, '');
        return {
          'title': subitem.title[0],
          'authors': authors,
          'url': subitem.link[0]
        };
      });
    }));
  });
};

Callbacks.prototype.getGithubCommits = function (options) {
  'use strict';
  var github = new GitHubApi({ protocol: 'https' });
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

Callbacks.prototype.get500pxPhotos = function (options) {
  'use strict';
  var api500 = new Api500px(options.key);
  var getPhotos = Promise.promisify(api500.photos.getByUsername, { context: api500.photos });
  return getPhotos(options.user, { sort: 'created_at' }).then(function (result) {
    return [].concat.apply([], result.photos.map(function (item) {
      return {
        'url': 'http://500px.com' + item.url,
        'title': item.name,
        'date': item.taken_at,
        'camera': item.camera
      };
    }));
  }).catch(function (err) {
    return { 'error': err };
  });
};

module.exports = new Callbacks();
