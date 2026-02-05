use actix_web::{App, HttpResponse, HttpServer, Responder, ResponseError, http::StatusCode, web};
use actix_web_rust_embed_responder::{EmbedResponse, EmbedableFileResponse, IntoResponse};
use rust_embed_for_web::RustEmbed;
use std::time::Duration;

mod middleware {
    pub use actix_web::middleware::*;
    pub use etag_actix_middleware::*;
}

mod actix {
    pub mod cache;
    pub mod error_handlers;
}

mod api {
    pub mod github;
    pub mod goodreads;
    pub mod lastfm;
}

#[derive(RustEmbed)]
#[folder = "public/"]
struct Embedded;

struct Goodreads;
struct LastFm;

struct ApiKey<T> {
    secret: redact::Secret<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ApiKey<T> {
    pub fn from_env(var: &str) -> Result<Self, dotenvy::Error> {
        dotenvy::var(var).map(Into::into).map(|s| Self {
            secret: s,
            _phantom: Default::default(),
        })
    }
}

impl<T> std::ops::Deref for ApiKey<T> {
    type Target = redact::Secret<String>;
    fn deref(&self) -> &redact::Secret<String> {
        &self.secret
    }
}

impl<T> Clone for ApiKey<T> {
    fn clone(&self) -> Self {
        Self {
            secret: self.secret.clone(),
            _phantom: Default::default(),
        }
    }
}

#[actix_web::route("/currently-reading.json", method = "GET", method = "HEAD")]
async fn currently_reading(
    api_key: web::Data<ApiKey<Goodreads>>,
    cache: web::Data<actix::cache::Cache>,
) -> Result<impl Responder, impl ResponseError> {
    let api_key = api_key.get_ref().clone();
    let duration = Duration::from_hours(24);
    let response = async move { api::goodreads::get_books(&api_key).await };
    cache.json("books", duration, response).await
}

#[actix_web::route("/recent-commits.json", method = "GET", method = "HEAD")]
async fn recent_commits(
    octocrab: web::Data<octocrab::Octocrab>,
    cache: web::Data<actix::cache::Cache>,
) -> Result<impl Responder, impl ResponseError> {
    let octocrab = octocrab.clone();
    let duration = Duration::from_mins(5);
    let response = async move { api::github::get_commits(&octocrab).await };
    cache.json("commits", duration, response).await
}

#[actix_web::route("/recent-tracks.json", method = "GET", method = "HEAD")]
async fn recent_tracks(
    api_key: web::Data<ApiKey<LastFm>>,
    cache: web::Data<actix::cache::Cache>,
) -> Result<impl Responder, impl ResponseError> {
    let api_key = api_key.get_ref().clone();
    let duration = Duration::from_secs(150);
    let response = async move { api::lastfm::get_tracks(&api_key).await };
    cache.json("tracks", duration, response).await
}

#[actix_web::route("/health", method = "GET", method = "HEAD")]
async fn health() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().body("OK")
}

#[actix_web::route("/{path:.*}", method = "GET", method = "HEAD")]
async fn r#static(path: web::Path<String>) -> EmbedResponse<EmbedableFileResponse> {
    match path.as_str() {
        "300-latexhax.html" | "404.html" | "410.html" | "500.html" => None,
        "" => Embedded::get("index.html"),
        path => Embedded::get(path),
    }
    .into_response()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix::error_handlers::ErrorHandlersExt;

    // Load environment variables & set up logging
    let _ = dotenvy::dotenv();
    env_logger::init();

    // Configure rustls (mainly for octocrab)
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default rustls provider");

    // Various shared application data
    let cache = web::Data::new(actix::cache::Cache::default());
    let github_api = web::Data::new(octocrab::Octocrab::default());
    let goodreads_api_key = web::Data::new(
        ApiKey::<Goodreads>::from_env("GOODREADS_API_KEY").expect("Missing GOODREADS_API_KEY"),
    );
    let lastfm_api_key = web::Data::new(
        ApiKey::<LastFm>::from_env("LASTFM_API_KEY").expect("Missing LASTFM_API_KEY"),
    );

    // Configure the HTTP server with all routes
    let server = HttpServer::new(move || {
        // Simple routes (308 redirects, 410 gone)
        const LATEXBOK_URI: &str =
            "https://github.com/urdh/latexbok/releases/download/edition-2/latexbok-a4.pdf";
        let redirects = vec![
            web::resource("/archives/{path:.*}").to(async |_: web::Path<()>| HttpResponse::Gone()),
            web::resource("/portfolio/{path:.*}").to(async |_: web::Path<()>| HttpResponse::Gone()),
            web::resource("/autobrew").to(async |_: web::Path<()>| HttpResponse::Gone()),
            web::resource("/chslacite").to(async |_: web::Path<()>| HttpResponse::Gone()),
            web::resource("/posts/I-X/{path:.*}").to(async |_: web::Path<()>| HttpResponse::Gone()),
            web::resource("/atom.xml").to(async |_: web::Path<()>| {
                web::Redirect::to("https://blog.sigurdhsson.org/atom.xml").permanent()
            }),
            web::resource("/2012/11/{name:[^/\\.]+}").to(async |name: web::Path<String>| {
                web::Redirect::to(format!("https://blog.sigurdhsson.org/2012/11/{name}"))
                    .permanent()
            }),
            web::resource("/2014/04/{name:[^/\\.]+}").to(async |name: web::Path<String>| {
                web::Redirect::to(format!("https://blog.sigurdhsson.org/2014/04/{name}"))
                    .permanent()
            }),
            web::resource("/2014/09/{name:[^/\\.]+}").to(async |name: web::Path<String>| {
                web::Redirect::to(format!("https://blog.sigurdhsson.org/2014/09/{name}"))
                    .permanent()
            }),
            web::resource("/skrapport/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/skrapport/{path}"))
                    .permanent()
            }),
            web::resource("/dotfiles/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/dotfiles/{path}"))
                    .permanent()
            }),
            web::resource("/skmath/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/skmath/{path}"))
                    .permanent()
            }),
            web::resource("/latexbok/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/latexbok/{path}"))
                    .permanent()
            }),
            web::resource("/skdoc/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/skdoc/{path}"))
                    .permanent()
            }),
            web::resource("/chscite/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/chscite/{path}"))
                    .permanent()
            }),
            web::resource("/streck/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://projects.sigurdhsson.org/streck/{path}"))
                    .permanent()
            }),
            web::resource("/webboken/v2/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://webboken.github.io/{path}")).permanent()
            }),
            web::resource("/webboken/v2/{path:.*}").to(async |path: web::Path<String>| {
                web::Redirect::to(format!("https://webboken.github.io/{path}")).permanent()
            }),
            web::resource("/media/projects/latexbok/latexbok.pdf")
                .to(async |_: web::Path<()>| web::Redirect::to(LATEXBOK_URI).permanent()),
            web::resource("/latexbok/media/latexbok.pdf$")
                .to(async |_: web::Path<()>| web::Redirect::to(LATEXBOK_URI).permanent()),
            web::resource("/latexhax")
                .to(async |_: web::Path<()>| web::Redirect::to("/latexhax.html").permanent()),
            web::resource("/latexhax/")
                .to(async |_: web::Path<()>| web::Redirect::to("/latexhax.html").permanent()),
            web::resource("/latexhax/index.html")
                .to(async |_: web::Path<()>| web::Redirect::to("/latexhax.html").permanent()),
            web::resource("/projects/latexhax.html")
                .to(async |_: web::Path<()>| web::Redirect::to("/latexhax.html").permanent()),
        ];

        // The latexhax route is more complicated, returning a custom HTTP 300 page
        let latexhax = web::resource("/latexhax.html").to(async || {
            Embedded::get("300-latexhax.html")
                .into_response()
                .customize()
                .with_status(StatusCode::MULTIPLE_CHOICES)
        });

        // Configure the actual application
        App::new()
            .app_data(github_api.clone())
            .app_data(goodreads_api_key.clone())
            .app_data(lastfm_api_key.clone())
            .app_data(cache.clone())
            .wrap(middleware::ETag::strong())
            .wrap(middleware::Compress::default())
            .wrap(
                middleware::ErrorHandlers::new()
                    .embed_file(StatusCode::NOT_FOUND, Embedded::get("404.html"))
                    .embed_file(StatusCode::GONE, Embedded::get("410.html"))
                    .embed_file(StatusCode::INTERNAL_SERVER_ERROR, Embedded::get("500.html")),
            )
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::MergeOnly,
            ))
            .service(currently_reading)
            .service(recent_commits)
            .service(recent_tracks)
            .service(health)
            .service(latexhax)
            .service(redirects)
            .service(r#static)
    });

    // Finally, run the server!
    server.bind("0.0.0.0:80")?.run().await
}
