var app = require('koa')();
var staticCache = require('koa-static-cache');
var fileCache = require('koa-file-cache');

var pathToRegexp = require('path-to-regexp');
var Promise = require('bluebird');
var GitHubApi = require('github');
var GoodreadsApi = require('goodreads');
var LastfmApi = require('lastfmapi');
var Api500px = require('500px');
var path = require('path');
var fs = require('mz/fs');

function getRecentTracks() {
  var lastfm = new LastfmApi({
    api_key: process.env.LASTFM_API_KEY || '',
    secret: process.env.LASTFM_SECRET || ''
  });
  var getRecentTracks = Promise.promisify(lastfm.user.getRecentTracks, lastfm.user);
  return getRecentTracks({user: 'TinyGuy'}).then(function (result) {
    return [].concat.apply([], result['track'].map(function(item) {
      return {
        'artist': item['artist']['#text'],
        'title': item['name'],
        'url': item['url'],
        'date': new Date(item['date']['uts'] * 1000).toISOString()
      };
    }));
  }).catch(function() {
    return {};
  });
}

function getCurrentBook() {
  var goodreads = new GoodreadsApi.client({
    key: process.env.GOODREADS_API_KEY || '',
    secret: process.env.GOODREADS_SECRET || ''
  });
  var getBooks = Promise.promisify(goodreads.getSingleShelf, goodreads);
  // Shitty goodreads node module is shitty and doesn't "conform to node.js
  // convention of accepting a callback as last argument and calling that
  // callback with error as the first argument and success value on the
  // second argument", the callback only accepting one argument containing data.
  return getBooks({userID: '27549920', shelf: 'currently-reading'}).then(function(){
    // Data should be here, but it isn't.
  }).catch(function(result) {
    // Here's the data.
    return [].concat.apply([], result.GoodreadsResponse.books.map(function(item) {
      return item.book.map(function(subitem) {
        var authors = subitem.authors[0].author.map(function(subsubitem) {
          return subsubitem.name[0] +
            ((subsubitem.role[0] != '') ? (' (' + subsubitem.role[0] + ')') : '');
        }).reduce(function(prev, curr, idx, arr) {
          if(arr.length <= 1)
            return curr;
          if(idx == arr.length - 1)
            return prev + ' and ' + curr;
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
}

function getGithubCommits() {
  var github = new GitHubApi({version: '3.0.0', protocol: 'https'});
  var getEvents = Promise.promisify(github.events.getFromUserPublic, github.events);
  return getEvents({user: 'urdh'}).then(function(result) {
    return [].concat.apply([], result.filter(function(item) {
      return item['type'] == 'PushEvent';
    }).map(function(item) {
      return item['payload']['commits'].map(function(subitem) {
        return {
          'sha': subitem['sha'],
          'url': subitem['url'],
          'message': subitem['message'].split("\n")[0],
          'repo': item['repo']['name'],
          'date': item['created_at']
        };
      });
    }));
  }).catch(function() {
    return {};
  });
}

function get500pxPhotos() {
  var api500 = new Api500px(process.env.PX500_API_KEY || '');
  var getPhotos = Promise.promisify(api500.photos.getByUsername, api500.photos);
  return getPhotos('urdh', {sort: 'created_at'}).then(function(result) {
    return [].concat.apply([], result['photos'].map(function(item) {
      return {
        'url': 'http://500px.com' + item['url'],
        'title': item['name'],
        'date': item['taken_at'],
        'camera': item['camera']
      };
    }));
  }).catch(function() {
    return {};
  });
}

// First, some "top-layer" middlewares
app.use(require('koa-helmet').defaults());
app.use(require('koa-conditional-get')());
app.use(require('koa-etag')());
app.use(require('koa-compress')());

// Then we handle errors
app.use(function *(next) {
  try {
    yield next;
    if(this.status == 404) {
      this.body = fs.readFileSync(path.join(__dirname, 'errors', '404.html'));
      this.type = 'html';
      this.status = 404;
    }
    if(this.status == 410) {
      this.body = fs.readFileSync(path.join(__dirname, 'errors', '410.html'));
      this.type = 'html';
      this.status = 410;
    }
  } catch (err) {
    this.status = err.status || 500;
    if(this.status == 500) {
      this.body = fs.readFileSync(path.join(__dirname, 'errors', '500.html'));
      this.type = 'html';
      this.status = 500;
    }
    console.log(err);
    this.app.emit('error', err, this);
  }
});

// For caching the expensive API calls
app.use(function *(next) {
  this.caching = /\.json$/.test(this.path);
  this.cacheName = this.path.replace(/\/+/, "") || 'not-cached';
  console.log(this.cacheName, this.caching, this.path);
  yield next;
});
app.use(fileCache({
  cacheTime: 5 * 60 * 1000,
  folder: '/tmp',
  gzip: false,
  delegate: true
}));

// This is just providing a very limited parts of some APIs
app.use(function *(next) {
  var lfmre = pathToRegexp('/recent-tracks.json');
  var grre = pathToRegexp('/currently-reading.json');
  var ghre = pathToRegexp('/recent-commits.json');
  var pxre = pathToRegexp('/recent-photos.json');
  if(lfmre.exec(this.path)) {
    if(!this.body) this.body = yield getRecentTracks();
    this.type = 'json';
  } else if(grre.exec(this.path)) {
    if(!this.body) this.body = yield getCurrentBook();
    this.type = 'json';
  } else if(ghre.exec(this.path)) {
    if(!this.body) this.body = yield getGithubCommits();
    this.type = 'json';
  } else if(pxre.exec(this.path)) {
    if(!this.body) this.body = yield get500pxPhotos();
    this.type = 'json';
  } else {
    yield next;
  }
});

// Then, our route middlewares for redirects and missing pages
function gone(uri) {
  return function*(next) {
    var re = pathToRegexp(uri);
    if(re.exec(this.path)) {
      this.status = 410;
    } else {
      yield next;
    }
  }
}
function moved(uri, target) {
  return function *(next){
    var re = pathToRegexp(uri);
    if(m = re.exec(this.path)) {
      this.set('Location', this.path.replace(re, target));
      this.status = 301;
    } else {
      yield next;
    }
  }
}
function multiple(uri, ident) {
  return function *(next){
    var re = pathToRegexp(uri);
    if(re.exec(this.path)) {
      this.body = fs.readFileSync(path.join(__dirname, 'errors', '300-' + ident + '.html'));
      this.type = 'html';
      this.status = 300;
    } else {
      yield next;
    }
  }
}
// These are gone forever
app.use(gone('/archives/*'));
app.use(gone('/portfolio/*'));
app.use(gone('/autobrew'));
app.use(gone('/chslacite'));
app.use(gone('/posts/I-X/*'));
// These are moved
app.use(moved('/webboken/v2/:uri*', 'http://webboken.github.io/$1'));
app.use(moved('/media/projects/latexbok/latexbok.pdf',
  'http://github.com/urdh/latexbok/releases/download/edition-2/latexbok-a4.pdf'));
app.use(moved('/latexbok/media/latexbok.pdf',
  'http://github.com/urdh/latexbok/releases/download/edition-2/latexbok-a4.pdf'));
app.use(moved('/latexhax/index.html', '/latexhax.html'));
app.use(moved('/projects/latexhax.html', '/latexhax.html'));
app.use(multiple('/latexhax.html', 'latexhax'));
// These are on the current blog
app.use(moved('/atom.xml',      'http://blog.sigurdhsson.org/atom.xml'));
app.use(moved('/2012/11/:post', 'http://blog.sigurdhsson.org/2012/11/$1'));
app.use(moved('/2014/04/:post', 'http://blog.sigurdhsson.org/2014/04/$1'));
app.use(moved('/2014/09/:post', 'http://blog.sigurdhsson.org/2014/09/$1'));
// Project on github from before move
app.use(moved('/skrapport/:uri*', 'http://projects.sigurdhsson.org/skrapport/$1'));
app.use(moved('/dotfiles/:uri*',  'http://projects.sigurdhsson.org/dotfiles/$1'));
app.use(moved('/skmath/:uri*',    'http://projects.sigurdhsson.org/skmath/$1'));
app.use(moved('/latexbok/:uri*',  'http://projects.sigurdhsson.org/latexbok/$1'));
app.use(moved('/skdoc/:uri*',     'http://projects.sigurdhsson.org/skdoc/$1'));
app.use(moved('/chscite/:uri*',   'http://projects.sigurdhsson.org/chscite/$1'));
app.use(moved('/streck/:uri*',    'http://projects.sigurdhsson.org/streck/$1'));

// Finally, the static cache serving middleware serving the hCard
app.use(staticCache(path.join(__dirname, 'public'), {
  maxAge: 28 * 24 * 60 * 60,
  buffer: process.env.DYNT ? true : false,
  gzip: false, // compress middleware does this
  alias: { '/': '/index.html' }
}));

// And run the application
if (!module.parent) app.listen(process.env.PORT || 5000);
