const callbacks = require('./callbacks.js');
const { pathToRegexp } = require('path-to-regexp');
const path = require('path');
const fs = require('mz/fs');

// First, some "top-layer" middlewares
let app = new (require('koa'))();
app.use(require('koa-helmet')());
app.use(require('koa-conditional-get')());
app.use(require('koa-etag')());
app.use(require('koa-compress')());

// Then we handle errors
app.use(async (ctx, next) => {
  try {
    await next();
    if (ctx.status == 404) {
      ctx.body = await fs.readFile(path.join(__dirname, 'errors', '404.html'));
      ctx.type = 'html';
      ctx.status = 404;
    }
    if (ctx.status == 410) {
      ctx.body = await fs.readFile(path.join(__dirname, 'errors', '410.html'));
      ctx.type = 'html';
      ctx.status = 410;
    }
  } catch (err) {
    ctx.status = err.status || 500;
    if (ctx.status == 500) {
      ctx.body = await fs.readFile(path.join(__dirname, 'errors', '500.html'));
      ctx.type = 'html';
      ctx.status = 500;
    }
    ctx.app.emit('error', err, this);
  }
});

// For caching the expensive API calls
app.use(async (ctx, next) => {
  ctx.caching = /.json$/.test(ctx.path);
  // eslint-disable-next-line optimize-regex/optimize-regex
  ctx.cacheName = ctx.path.replace(/\/+/, '') || 'not-cached';
  await next();
});
app.use(require('koa-file-cache')({
  cacheTime: 5 * 60 * 1000,
  folder: '/tmp',
  gzip: false,
  delegate: true
}));

// This is just providing a very limited parts of some APIs
app.use(async (ctx, next) => {
  const lfmre = pathToRegexp('/recent-tracks.json');
  const grre = pathToRegexp('/currently-reading.json');
  const ghre = pathToRegexp('/recent-commits.json');
  const pxre = pathToRegexp('/recent-photos.json');
  if (lfmre.exec(ctx.path)) {
    if (!ctx.body) ctx.body = await callbacks.getRecentTracks({
      key: process.env.LASTFM_API_KEY || '',
      secret: process.env.LASTFM_SECRET || '',
      user: 'TinyGuy'
    });
    ctx.type = 'json';
  } else if (grre.exec(ctx.path)) {
    if (!ctx.body) ctx.body = await callbacks.getCurrentBook({
      key: process.env.GOODREADS_API_KEY || '',
      secret: process.env.GOODREADS_SECRET || '',
      user: '27549920'
    });
    ctx.type = 'json';
  } else if (ghre.exec(ctx.path)) {
    if (!ctx.body) ctx.body = await callbacks.getGithubCommits({
      user: 'urdh'
    });
    ctx.type = 'json';
  } else if (pxre.exec(ctx.path)) {
    if (!ctx.body) ctx.body = await callbacks.getPhotos();
    ctx.type = 'json';
  } else {
    await next();
  }
});

// Then, our route middlewares for redirects and missing pages
function gone(uri) {
  return async (ctx, next) => {
    const re = pathToRegexp(uri);
    if (re.exec(ctx.path)) {
      ctx.status = 410;
    } else {
      await next();
    }
  };
}
function moved(uri, target) {
  return async (ctx, next) => {
    var re = pathToRegexp(uri);
    if (re.exec(ctx.path)) {
      ctx.set('Location', ctx.path.replace(re, target));
      ctx.status = 301;
    } else {
      await next();
    }
  };
}
function multiple(uri, ident) {
  return async (ctx, next) => {
    const re = pathToRegexp(uri);
    if (re.exec(ctx.path)) {
      ctx.body = fs.readFileSync(path.join(__dirname, 'errors', '300-' + ident + '.html'));
      ctx.type = 'html';
      ctx.status = 300;
    } else {
      await next();
    }
  };
}

// These are gone forever
app.use(gone('/archives/:uri*'));
app.use(gone('/portfolio/:uri*'));
app.use(gone('/autobrew'));
app.use(gone('/chslacite'));
app.use(gone('/posts/I-X/:uri*'));
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
app.use(moved('/atom.xml', 'http://blog.sigurdhsson.org/atom.xml'));
app.use(moved('/2012/11/:post', 'http://blog.sigurdhsson.org/2012/11/$1'));
app.use(moved('/2014/04/:post', 'http://blog.sigurdhsson.org/2014/04/$1'));
app.use(moved('/2014/09/:post', 'http://blog.sigurdhsson.org/2014/09/$1'));
// Project on github from before move
app.use(moved('/skrapport/:uri*', 'http://projects.sigurdhsson.org/skrapport/$1'));
app.use(moved('/dotfiles/:uri*', 'http://projects.sigurdhsson.org/dotfiles/$1'));
app.use(moved('/skmath/:uri*', 'http://projects.sigurdhsson.org/skmath/$1'));
app.use(moved('/latexbok/:uri*', 'http://projects.sigurdhsson.org/latexbok/$1'));
app.use(moved('/skdoc/:uri*', 'http://projects.sigurdhsson.org/skdoc/$1'));
app.use(moved('/chscite/:uri*', 'http://projects.sigurdhsson.org/chscite/$1'));
app.use(moved('/streck/:uri*', 'http://projects.sigurdhsson.org/streck/$1'));

// Finally, the static cache serving middleware serving the hCard
app.use(require('koa-static-cache')(path.join(__dirname, 'public'), {
  maxAge: 28 * 24 * 60 * 60,
  buffer: process.env.DYNT ? true : false,
  gzip: false, // compress middleware does this
  alias: { '/': '/index.html' }
}));

// And run the application
if (!module.parent) app.listen(process.env.PORT || 5000);
