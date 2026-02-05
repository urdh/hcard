use actix_web::{Result, body, dev, http, middleware};
use rust_embed_for_web::EmbedableFile;

pub trait ErrorHandlersExt
where
    Self: Sized,
{
    fn embed_file<T>(self, status: http::StatusCode, file: Option<T>) -> Self
    where
        T: EmbedableFile + 'static,
        <T as EmbedableFile>::Data: body::MessageBody;
}

impl<B> ErrorHandlersExt for middleware::ErrorHandlers<B>
where
    B: body::MessageBody + 'static,
{
    fn embed_file<T>(self, status: http::StatusCode, file: Option<T>) -> Self
    where
        T: EmbedableFile + 'static,
        <T as EmbedableFile>::Data: body::MessageBody,
    {
        if let Some(file) = file {
            self.handler(status, move |res| custom_error_handler(res, &file))
        } else {
            self
        }
    }
}

fn custom_error_handler<B, T>(
    res: dev::ServiceResponse<B>,
    file: &T,
) -> Result<middleware::ErrorHandlerResponse<B>>
where
    B: body::MessageBody + 'static,
    T: EmbedableFile + 'static,
    <T as EmbedableFile>::Data: body::MessageBody,
{
    let (req, mut resp) = res.into_parts();

    // Avoid overriding errors which already have contents
    let resp = match resp.body().size() {
        body::BodySize::Sized(0) | body::BodySize::None => {
            // Override the content type to match the embedable file
            if let Some(mime_type) = file
                .mime_type()
                .and_then(|t| http::header::HeaderValue::from_str(t.as_ref()).ok())
            {
                resp.headers_mut()
                    .insert(http::header::CONTENT_TYPE, mime_type);
            }

            // Override the actual contents of the error as well
            resp.set_body(file.data()).map_into_boxed_body()
        }
        _ => resp.map_into_boxed_body(),
    };

    Ok(middleware::ErrorHandlerResponse::Response(
        dev::ServiceResponse::new(req, resp)
            .map_into_boxed_body()
            .map_into_right_body(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header::{ContentType, HeaderValue, TryIntoHeaderValue};
    use actix_web::http::{StatusCode, header};
    use actix_web::middleware::ErrorHandlers;
    use actix_web::{App, HttpResponse, test, web};

    #[derive(Clone, Copy)]
    struct TestFile;

    impl EmbedableFile for TestFile {
        type Data = &'static [u8];
        type Meta = &'static str;

        fn name(&self) -> Self::Meta {
            "TEST FILE"
        }

        fn data(&self) -> Self::Data {
            b"TEST DATA"
        }

        fn data_gzip(&self) -> Option<Self::Data> {
            None
        }

        fn data_br(&self) -> Option<Self::Data> {
            None
        }

        fn last_modified_timestamp(&self) -> Option<i64> {
            None
        }

        fn last_modified(&self) -> Option<Self::Meta> {
            None
        }

        fn hash(&self) -> Self::Meta {
            r"_G][cDmoFa^pER]PfR:\<8^t]r[f'3hOLXDmih4^"
        }

        fn etag(&self) -> Self::Meta {
            r#""_G][cDmoFa^pER]PfR:\<8^t]r[f'3hOLXDmih4^""#
        }

        fn mime_type(&self) -> Option<Self::Meta> {
            Some("text/plain")
        }
    }

    #[actix_web::test]
    async fn replaces_only_empty_body() {
        let file = TestFile;
        let app = test::init_service({
            App::new()
                .wrap(ErrorHandlers::new().embed_file(StatusCode::GONE, Some(file)))
                .route("/gone", web::get().to(HttpResponse::Gone))
                .route("/unauth", web::get().to(HttpResponse::Unauthorized))
                .route(
                    "/nonempty",
                    web::get().to(async move || HttpResponse::Gone().json(vec!["some tasty json"])),
                )
        })
        .await;

        // Replaces contents on matching status code if empty body
        let req = test::TestRequest::default().uri("/gone").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::GONE);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE),
            file.mime_type().map(HeaderValue::from_static).as_ref()
        );
        assert_eq!(test::read_body(res).await, file.data());

        // Does not replace contents on other status codes
        let req = test::TestRequest::default().uri("/unauth").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert!(res.headers().get(header::CONTENT_TYPE).is_none());
        assert_ne!(test::read_body(res).await, file.data());

        // Does not replace contents on matching status code if body is non-empty
        let req = test::TestRequest::default().uri("/nonempty").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::GONE);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE),
            ContentType::json().try_into_value().ok().as_ref()
        );
        assert_ne!(test::read_body(res).await, file.data());
    }
}
