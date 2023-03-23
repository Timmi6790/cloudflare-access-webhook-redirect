use thiserror::Error;
use tokio_stream::StreamExt;

pub struct ActixToReqwestConverter {}

pub type ConverterResult<T> = anyhow::Result<T, ConverterError>;

impl ActixToReqwestConverter {
    fn is_valid_header_name(name: &str) -> bool {
        !matches!(name, "host")
    }

    pub async fn convert_body(
        payload: &mut actix_web::web::Payload,
    ) -> ConverterResult<reqwest::Body> {
        let mut bytes = actix_web::web::BytesMut::new();
        while let Some(item) = payload.next().await {
            let item = item?;
            bytes.extend_from_slice(&item);
        }

        let body = reqwest::Body::from(bytes.freeze());
        Ok(body)
    }

    pub async fn convert_headers(
        headers: &actix_web::http::header::HeaderMap,
        additional_headers: usize,
    ) -> ConverterResult<reqwest::header::HeaderMap> {
        println!("Converting headers: {:?}", headers);

        let mut target_headers: reqwest::header::HeaderMap =
            reqwest::header::HeaderMap::with_capacity(headers.capacity() + additional_headers);
        headers
            .iter()
            .filter(|(key, _)| ActixToReqwestConverter::is_valid_header_name(key.as_str()))
            .for_each(|(key, value)| {
                if let Ok(value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                    target_headers.append(key, value);
                }
            });

        Ok(target_headers)
    }
}

#[derive(Error, Debug)]
pub enum ConverterError {
    #[error("Payload Error")]
    Config(#[from] actix_web::error::PayloadError),
}

impl From<ConverterError> for actix_web::Error {
    fn from(e: ConverterError) -> Self {
        actix_web::error::ErrorBadRequest(e)
    }
}
