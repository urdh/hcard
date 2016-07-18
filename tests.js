var fs = require('fs');
var test = require('tape');
var lint = require('html5-lint');
var blc = require('broken-link-checker');
var JSHINT = require('jshint').JSHINT;

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
    'tests.js'
  ]
};

test('HTML5-lint', function (t) {
  files.html.forEach(function (file) {
    t.test('HTML5-lint: ' + file, function (st) {
      fs.readFile(file, 'utf8', function (err, html) {
        if(err) {
          st.fail('Could not read file!');
          return;
        }

        lint(html, function (err, results) {
          results.messages.forEach(function (msg) {
            st.comment('Linter ' + msg.type + ': ' + msg.message);
          });
          st.equal(results.messages.length, 0, 'No HTML5-lint messages in ' + file);
        });
      });
      st.end();
    });
  });
  t.end();
});

test('Broken links', function (t) {
  var exceptions = [
    'https://linkedin.com/in/simonsigurdhsson/'
  ];
  files.html.forEach(function (file) {
    t.test('Broken links: ' + file, function (st) {
      fs.readFile(file, 'utf8', function (err, html) {
        var checker = new blc.HtmlChecker({
          'honorRobotExclusions': false,
          'excludeInternalLinks': true
        }, {
          html: function (tree, robots){},
          junk: function (result){
            if(result.excluded) {
              st.skip(result.url.original + ' (' + blc[result.excludedReason] + ')');
            }
          },
          link: function (result){
            if(result.broken) {
              if(result.brokenReason === "BLC_INVALID") {
                st.skip(result.url.original + ' (' + blc[result.brokenReason] + ')');
              } else if(exceptions.indexOf(result.url.original) != -1) {
                st.skip(result.url.original + ' (Exception list)');
              } else {
                st.fail(result.url.original + ' (' + blc[result.brokenReason] + ')');
              }
            } else {
              st.pass(result.url.original);
            }
          },
          complete: function (){}
        });
        checker.scan(html, '');
      });
      st.end();
    });
  });
  t.end();
});

test('JSHint', function (t) {
  files.js.forEach(function (file) {
    t.test('JSHint: ' + file, function (st) {
      fs.readFile(file, 'utf8', function (err, js) {
        if(err) {
          st.fail('Could not read file!');
          return;
        }

        // TODO: read the options from .jshintrc
        var ok = JSHINT(js, {
          "indent":    2,
          "browser":   false,
          "node":      true,
          "esversion": 6
        });
        var result = JSHINT.data();
        result.errors = result.errors || [];

        function row(e) {
            return e.line;
        }
        function col(e) {
            return (e.character || e.column);
        }
        function message(e) {
            return e.reason + '(' + e.code + ')';
        }

        result.errors.forEach(function (e) {
          st.comment('Linter error: ' + file + ':' + row(e) + ':' + col(e) + ': ' + message(e));
        });
        st.equal(result.errors.length, 0, 'No JSHint messages in ' + file);
      });
      st.end();
    });
  });
  t.end();
});
