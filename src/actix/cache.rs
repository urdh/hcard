use actix_web::{HttpResponse, ResponseError, http, web};
use derive_more::{Display, Error};
use std::time::Duration;

type CacheKey = &'static str;

#[derive(Clone, Debug)]
struct CacheValue(serde_json::Value);

impl<T: serde::Serialize> TryFrom<web::Json<T>> for CacheValue {
    type Error = serde_json::Error;
    fn try_from(value: web::Json<T>) -> Result<Self, Self::Error> {
        serde_json::to_value(value).map(Self)
    }
}

impl From<CacheValue> for web::Json<serde_json::Value> {
    fn from(CacheValue(value): CacheValue) -> Self {
        web::Json(value)
    }
}

#[derive(Debug, Display, Error)]
pub enum CacheError<T>
where
    T: ResponseError,
{
    Inner(T),
    Serde(serde_json::Error),
}

impl<T> ResponseError for CacheError<T>
where
    T: ResponseError,
{
    fn error_response(&self) -> HttpResponse {
        match self {
            CacheError::Inner(err) => err.error_response(),
            CacheError::Serde(err) => err.error_response(),
        }
    }
    fn status_code(&self) -> http::StatusCode {
        match self {
            CacheError::Inner(err) => err.status_code(),
            CacheError::Serde(err) => err.status_code(),
        }
    }
}

pub struct Cache(minicache::MiniCache<CacheKey, CacheValue>);

impl Cache {
    pub async fn json<Fut, T, E>(
        &self,
        key: &'static str,
        duration: Duration,
        response: Fut,
    ) -> Result<web::Json<serde_json::Value>, CacheError<E>>
    where
        Fut: Future<Output = Result<web::Json<T>, E>>,
        T: serde::Serialize,
        E: ResponseError + Send + Sync + 'static,
    {
        use futures::FutureExt;
        use futures::TryFutureExt;
        let cache_op = response
            .map_err(CacheError::Inner)
            .and_then(async |value| CacheValue::try_from(value).map_err(CacheError::Serde))
            .and_then(async |value| {
                self.0
                    .set(key, value.clone(), Some(duration))
                    .then(async move |_| Ok(value))
                    .await
            });
        self.0
            .get(&key)
            .then(async |v: Option<_>| match v {
                None => cache_op.await,
                Some(value) => Ok(value),
            })
            .ok_into()
            .await
    }
}

impl Default for Cache {
    fn default() -> Cache {
        Self(minicache::MiniCache::new(Duration::from_secs(10)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::TryFutureExt;
    use std::convert::Infallible;

    #[derive(Debug, Display, Error)]
    struct TestError;
    impl ResponseError for TestError {}

    #[actix_web::test]
    async fn ok_is_cached() -> Result<(), CacheError<Infallible>> {
        let cache = Cache::default();
        let duration = Duration::from_secs(1);
        let response = async { Ok(web::Json(vec![42])) };
        assert_eq!(
            cache
                .json("key", duration, response)
                .map_ok(web::Json::into_inner)
                .await?,
            serde_json::json!(vec![42])
        );
        assert!(cache.0.contains(&"key").await);
        Ok(())
    }

    #[actix_web::test]
    async fn err_is_not_cached() {
        let cache = Cache::default();
        let duration = Duration::from_secs(1);
        let response = async { Err::<web::Json<()>, _>(TestError) };
        assert!(cache.json("key", duration, response).await.is_err());
        assert!(!cache.0.contains(&"key").await);
    }

    #[actix_web::test]
    async fn uses_cached_value() -> Result<(), CacheError<Infallible>> {
        let cache = Cache::default();
        let duration = Duration::from_secs(1);
        let response = async { panic!("should use cached value") };
        let value = web::Json(vec![42]).try_into().map_err(CacheError::Serde)?;
        cache.0.set("key", value, None).await;
        assert_eq!(
            cache
                .json::<_, web::Json<()>, std::convert::Infallible>("key", duration, response)
                .map_ok(web::Json::into_inner)
                .await?,
            serde_json::json!(vec![42])
        );
        Ok(())
    }

    #[actix_web::test]
    async fn cache_expires_as_expected() -> Result<(), CacheError<Infallible>> {
        let cache = Cache::default();
        let duration = Duration::from_millis(125);
        let value = web::Json(vec![42]).try_into().map_err(CacheError::Serde)?;
        cache.0.set("key", value, Some(duration)).await;
        actix_web::rt::time::sleep(Duration::from_millis(250)).await;
        assert!(!cache.0.contains(&"key").await);
        Ok(())
    }
}
