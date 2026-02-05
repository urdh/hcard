use actix_web::{HttpResponse, error, http, web};
use chrono::{DateTime, Utc};
use derive_more::{Display, Error, From};

pub const LASTFM_USER: &str = "TinyGuy";

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Track {
    pub artist: String,
    pub title: String,
    pub url: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display("Could not query last.fm ({})", source)]
    LastFm { source: lastfm::errors::Error },
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(serde_json::json!({
          "error": self.to_string()
        }))
    }
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn get_tracks(api_key: &redact::Secret<String>) -> Result<web::Json<Vec<Track>>, Error> {
    use futures::{StreamExt, TryFutureExt, TryStreamExt};
    use lastfm::client::RecentTracksFetcher;
    lastfm::Client::builder()
        .api_key(api_key.expose_secret())
        .username(LASTFM_USER)
        .build()
        .recent_tracks(None, None)
        .map_ok(RecentTracksFetcher::into_stream)
        .try_flatten_stream()
        .map_ok(|track| Track {
            artist: track.artist.name,
            title: track.name,
            url: track.url,
            date: track.date,
        })
        .take(5)
        .try_collect::<Vec<_>>()
        .map_ok(web::Json)
        .err_into()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn make_api_request() -> Result<(), Error> {
        let _ = dotenvy::dotenv();
        assert!(
            !get_tracks(
                &dotenvy::var("LASTFM_API_KEY")
                    .expect("Missing API key")
                    .into()
            )
            .await?
            .is_empty()
        );
        Ok(())
    }
}
