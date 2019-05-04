var fs = require('fs');
var tap = require('tap');
var lint = require('html5-lint');
var blc = require('broken-link-checker');
var ESLint = require('eslint').CLIEngine;
var callbacks = require('./callbacks.js');
var index = require('./index.js'); // eslint-disable-line no-unused-vars

var files = {
  'html': [
    'public/index.html',
    'errors/300-latexhax.html',
    'errors/404.html',
    'errors/410.html',
    'errors/500.html'
  ],
  'robots': ['public/robots.txt'],
  'sitemap': ['public/sitemap.xml'],
  'js': [
    'index.js',
    'callbacks.js',
    'tests.js'
  ]
};

tap.test('HTML5-lint', function (t) {
  t.plan(files.html.length);
  files.html.forEach(function (file) {
    t.test('HTML5-lint: ' + file, function (st) {
      fs.readFile(file, 'utf8', function (err, html) {
        if(err) {
          throw new Error(err);
        }

        lint(html, function (err, results) {
          if(err) {
            st.assert(err, 'Cannot lint: ' + err.message, {skip: true});
          } else {
            st.equal(results.messages.length, 0, 'No HTML5-lint messages in ' + file);
          }
          st.end();
        });
      });
    });
  });
});

tap.test('Broken links', function (t) {
  var exceptions = [
    'https://linkedin.com/in/simonsigurdhsson/'
  ];
  t.plan(files.html.length);
  files.html.forEach(function (file) {
    t.test('Broken links: ' + file, function (st) {
      fs.readFile(file, 'utf8', function (err, html) {
        if(err) {
          throw new Error(err);
        }

        var checker = new blc.HtmlChecker({
          'honorRobotExclusions': false,
          'excludeInternalLinks': true
        }, {
          html: function (_tree, _robots){ },
          junk: function (result){
            if(result.excluded) {
              st.assert(result.excluded, result.url.original + ' (' + blc[result.excludedReason] + ')', {skip: true});
            }
          },
          link: function (result){
            if(result.broken) {
              if(result.brokenReason === 'BLC_INVALID') {
                st.assert(result.broken, result.url.original + ' (' + blc[result.brokenReason] + ')', {skip: true});
              } else if(exceptions.indexOf(result.url.original) != -1) {
                st.assert(result.broken, result.url.original + ' (Exception list)', {skip: true});
              } else {
                st.assert(result.broken, result.url.original + ' (' + blc[result.brokenReason] + ')');
              }
            } else {
              st.assertNot(result.broken, result.url.original);
            }
          },
          complete: function (){
            st.end();
          }
        });
        checker.scan(html, '');
      });
    });
  });
});

tap.test('ESLint', function (t) {
  var eslint = new ESLint({});

  t.plan(files.js.length);
  files.js.forEach(function (file) {
    t.test('ESLint: ' + file, function (st) {
      var report = eslint.executeOnFiles([file]);

      var errors = report.results.map(function(result) {
        return result.errorCount;
      }).reduce(function (a, b) { return a + b; }, 0);

      var warnings = report.results.map(function(result) {
        return result.warningCount;
      }).reduce(function (a, b) { return a + b; }, 0);

      var issues = errors + warnings;
      st.equal(issues, 0, 'No ESLint messages in ' + file);
      st.end();
    });
  });
});

if(process.env.LASTFM_API_KEY && process.env.LASTFM_SECRET) {
  tap.test('Last.fm API proxy', function(t) {
    callbacks.getRecentTracks({
      key:    process.env.LASTFM_API_KEY || '',
      secret: process.env.LASTFM_SECRET  || '',
      user:   'TinyGuy'
    }).then(function(result) {
      t.type(result, Array);
      t.notEqual(result.length, 0);
      t.notEqual(result[0].url, undefined);
      t.notEqual(result[0].title, undefined);
      t.notEqual(result[0].artist, undefined);
      t.notEqual(result[0].date, undefined);
      t.end();
    });
  });
}

if(process.env.GOODREADS_API_KEY && process.env.GOODREADS_SECRET) {
  tap.test('Goodreads API proxy', function(t) {
    callbacks.getCurrentBook({
      key:    process.env.GOODREADS_API_KEY || '',
      secret: process.env.GOODREADS_SECRET  || '',
      user:   '27549920'
    }).then(function(result) {
      t.type(result, Array);
      if(result.length > 0) {
        t.notEqual(result[0].url, undefined);
        t.notEqual(result[0].title, undefined);
        t.notEqual(result[0].authors, undefined);
      }
      t.end();
    });
  });
}

tap.test('Github API proxy', function(t) {
  callbacks.getGithubCommits({
    user: 'urdh'
  }).then(function(result) {
    t.type(result, Array);
    t.notEqual(result.length, 0);
    t.notEqual(result[0].sha, undefined);
    t.notEqual(result[0].url, undefined);
    t.notEqual(result[0].message, undefined);
    t.notEqual(result[0].repo, undefined);
    t.notEqual(result[0].date, undefined);
    t.end();
  });
});

if(process.env.PX500_API_KEY) {
  tap.todo('500px API proxy', function(t) {
    callbacks.get500pxPhotos({
    }).then(function(result) {
      t.type(result, Array);
      t.equal(result.length, 0);
      // t.notEqual(result[0].url, undefined);
      // t.notEqual(result[0].title, undefined);
      // t.notEqual(result[0].camera, undefined);
      // t.notEqual(result[0].date, undefined);
      t.end();
    });
  });
}
