var app = require('koa')();
var staticCache = require('koa-static-cache');

var pathToRegexp = require('path-to-regexp');
var Promise = require('bluebird');
var GitHubApi = require('github');
var path = require('path');
var fs = require('mz/fs');

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
    this.app.emit('error', err, this);
  }
});

// This is just providing a very limited part of the Github API
app.use(function *(next) {
  var re = pathToRegexp('/recent-commits.json');
  if(re.exec(this.path)) {
    this.body = yield getGithubCommits();
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
