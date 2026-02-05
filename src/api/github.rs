use actix_web::{HttpResponse, error, http, web};
use chrono::{DateTime, Utc};
use derive_more::{Display, Error, From};

pub const GITHUB_USER: &str = "urdh";

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Commit {
    pub sha: String,
    pub url: String,
    pub message: String,
    pub repo: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display("Could not query Github API: {}", source)]
    Octocrab { source: octocrab::Error },

    #[display("Could not parse repository name '{}'", repo_name)]
    #[from(ignore)]
    BadRepoName { repo_name: String },
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

pub async fn get_commits(octocrab: &octocrab::Octocrab) -> Result<web::Json<Vec<Commit>>, Error> {
    use futures::{StreamExt, TryStreamExt, stream};
    use futures::{TryFutureExt, future, future::Either};
    use octocrab::models::events::{payload::*, *};
    octocrab
        .get(format!("/users/{}/events/public", GITHUB_USER), None::<&()>)
        .err_into()
        .map_ok(|events: Vec<serde_json::Value>| {
            stream::iter(events.into_iter())
                .filter_map(async |value| serde_json::from_value::<Event>(value).ok())
                .map(Ok)
        })
        .try_flatten_stream()
        .try_filter(|event| future::ready(event.r#type == EventType::PushEvent))
        .try_filter_map(|event| match event.payload.and_then(|p| p.specific) {
            Some(EventPayload::PushEvent(data)) => Either::Left(
                get_commit(octocrab, event.repo.name, data.head, event.created_at).map_ok(Some),
            ),
            _ => Either::Right(future::ready(Ok(None))),
        })
        .take(5)
        .try_collect::<Vec<_>>()
        .map_ok(web::Json)
        .await
}

async fn get_commit(
    octocrab: &octocrab::Octocrab,
    repo_name: String,
    commit_sha: String,
    created_at: DateTime<Utc>,
) -> Result<Commit, Error> {
    use futures::TryFutureExt;
    let (user, repo) = repo_name.split_once('/').ok_or(Error::BadRepoName {
        repo_name: repo_name.clone(),
    })?;
    octocrab
        .commits(user, repo)
        .get(commit_sha)
        .map_ok(|commit| Commit {
            sha: commit.sha,
            url: commit.html_url,
            message: commit
                .commit
                .message
                .lines()
                .map(Into::into)
                .next()
                .unwrap_or_default(),
            repo: repo_name,
            date: created_at,
        })
        .err_into()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn make_api_request() -> Result<(), Error> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Could not set up rustls");
        assert!(!get_commits(&octocrab::instance()).await?.is_empty());
        Ok(())
    }
}
