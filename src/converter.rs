use thiserror::Error;
use tokio_stream::StreamExt;

pub struct ActixToReqwestConverter {}

pub type ConverterResult<T> = anyhow::Result<T, ConverterError>;

impl ActixToReqwestConverter {
    fn is_valid_header_name(name: &str) -> bool {
        trace!("Checking for valid header name: {}", name);
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

    pub fn convert_headers(
        headers: &actix_web::http::header::HeaderMap,
        additional_headers: usize,
    ) -> reqwest::header::HeaderMap {
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

        target_headers
    }
}

pub struct ReqwestToActixConverter {}

impl ReqwestToActixConverter {
    pub fn convert_status_code(
        status_code: reqwest::StatusCode,
    ) -> ConverterResult<actix_web::http::StatusCode> {
        actix_web::http::StatusCode::from_u16(status_code.as_u16())
            .map_err(|_| ConverterError::invalid_status_code(status_code))
    }

    pub async fn convert_response(
        response: reqwest::Response,
    ) -> ConverterResult<actix_web::HttpResponse> {
        let status_code = ReqwestToActixConverter::convert_status_code(response.status())?;
        let body = response.bytes().await?;

        let response = actix_web::HttpResponse::build(status_code).body(body);
        Ok(response)
    }
}

#[derive(Error, Debug)]
pub enum ConverterError {
    #[error("Payload Error")]
    Payload(#[from] actix_web::error::PayloadError),
    #[error("Invalid Status Code")]
    InvalidStatusCode(String),
    #[error("Reqwest Error")]
    ReqwestError(#[from] reqwest::Error),
}

impl ConverterError {
    fn invalid_status_code(status_code: reqwest::StatusCode) -> Self {
        ConverterError::InvalidStatusCode(format!("Invalid status code: {}", status_code.as_u16()))
    }
}

impl From<ConverterError> for actix_web::Error {
    fn from(e: ConverterError) -> Self {
        actix_web::error::ErrorBadRequest(e)
    }
}

#[cfg(test)]
mod tests_actix_to_reqwest_converter {
    use std::collections::HashMap;

    fn convert_headers(values: HashMap<String, String>) -> actix_web::http::header::HeaderMap {
        let mut header_map = actix_web::http::header::HeaderMap::new();

        for (key, value) in values {
            let key = http::header::HeaderName::from_bytes(key.as_bytes()).unwrap();
            let value = http::header::HeaderValue::from_bytes(value.as_bytes()).unwrap();

            header_map.append(key, value);
        }

        header_map
    }

    // TODO: Add body converter tests

    #[test]
    fn test_convert_headers_invalid_header() {
        let mut header_values = HashMap::new();
        header_values.insert("Host".to_string(), "localhost".to_string());

        let headers = convert_headers(header_values);

        let converted_headers = super::ActixToReqwestConverter::convert_headers(&headers, 0);

        assert!(converted_headers.is_empty());
    }

    #[test]
    fn test_convert_headers() {
        let mut header_values = HashMap::new();
        // Valid headers
        header_values.insert("test".to_string(), "value".to_string());

        // Invalid headers
        header_values.insert("host".to_string(), "localhost".to_string());

        let headers = convert_headers(header_values);

        let converted_headers = super::ActixToReqwestConverter::convert_headers(&headers, 0);

        assert_eq!(converted_headers.len(), 1);
        assert_eq!(converted_headers.get("test").unwrap(), "value");
    }
}

#[cfg(test)]
mod tests_reqwest_to_actix_converter {
    use http::response::Builder;
    use reqwest::{Response, ResponseBuilderExt, Url};

    #[test]
    fn test_convert_status_code() {
        let status_code = reqwest::StatusCode::OK;
        let actix_status_code =
            super::ReqwestToActixConverter::convert_status_code(status_code).unwrap();

        assert_eq!(actix_status_code, http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_convert_response() {
        let url = Url::parse("https://example.com").unwrap();
        let response = Builder::new()
            .status(200)
            .url(url.clone())
            .body("foo")
            .unwrap();

        let response = Response::from(response);
        let actix_response = super::ReqwestToActixConverter::convert_response(response)
            .await
            .unwrap();

        assert_eq!(actix_response.status(), http::StatusCode::OK);

        // TODO: VERIFY BODY
    }
}
