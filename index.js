var callbacks = require('./callbacks.js');
var pathToRegexp = require('path-to-regexp');
var path = require('path');
var fs = require('mz/fs');

// First, some "top-layer" middlewares
var app = new (require('koa'))();
app.use(require('koa-helmet')());
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
  yield next;
});
app.use(require('koa-file-cache')({
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
    if(!this.body) this.body = yield callbacks.getRecentTracks({
      key:    process.env.LASTFM_API_KEY || '',
      secret: process.env.LASTFM_SECRET  || '',
      user:   'TinyGuy'
    });
    this.type = 'json';
  } else if(grre.exec(this.path)) {
    if(!this.body) this.body = yield callbacks.getCurrentBook({
      key:    process.env.GOODREADS_API_KEY || '',
      secret: process.env.GOODREADS_SECRET  || '',
      user:   '27549920'
    });
    this.type = 'json';
  } else if(ghre.exec(this.path)) {
    if(!this.body) this.body = yield callbacks.getGithubCommits({
      user: 'urdh'
    });
    this.type = 'json';
  } else if(pxre.exec(this.path)) {
    if(!this.body) this.body = yield callbacks.get500pxPhotos({
      key:  process.env.PX500_API_KEY || '',
      user: 'urdh'
    });
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
  };
}
function moved(uri, target) {
  return function *(next){
    var re = pathToRegexp(uri);
    if(re.exec(this.path)) {
      this.set('Location', this.path.replace(re, target));
      this.status = 301;
    } else {
      yield next;
    }
  };
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
  };
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
app.use(require('koa-static-cache')(path.join(__dirname, 'public'), {
  maxAge: 28 * 24 * 60 * 60,
  buffer: process.env.DYNT ? true : false,
  gzip: false, // compress middleware does this
  alias: { '/': '/index.html' }
}));

// And run the application
if (!module.parent) app.listen(process.env.PORT || 5000);
