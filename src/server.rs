use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::RegexSet;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;
use secrecy::{ExposeSecret, Secret};

use crate::converter::{ActixToReqwestConverter, ReqwestToActixConverter};
use crate::error::Error;
use crate::Result;

async fn get_health_status() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

// TODO: Add query support
// TODO: Add more method support?
async fn handle_web_hook(
    mut payload: web::Payload,
    request: HttpRequest,
    path: web::Path<String>,
    web_hook_data: web::Data<WebHookData>,
) -> core::result::Result<HttpResponse, actix_web::Error> {
    // Only allow specific paths
    info!("Received request for path: {}", path);
    if !web_hook_data.is_allowed_path(&path) {
        debug!("Path not allowed: {}", path);
        return Ok(HttpResponse::NotFound().finish());
    }

    // Craft target url
    let target_url = web_hook_data.get_target_url(path.as_str()).map_err(|e| {
        error!("Failed to join URL: {}", e);
        actix_web::error::ErrorBadRequest(e)
    })?;

    // Convert body
    let body = ActixToReqwestConverter::convert_body(&mut payload).await?;

    // Convert headers
    let mut target_headers: HeaderMap =
        ActixToReqwestConverter::convert_headers(request.headers(), 2);

    // Add Cloudflare Access headers
    target_headers.append("CF-Access-Client-Id", web_hook_data.access_id().clone());
    target_headers.append(
        "CF-Access-Client-Secret",
        web_hook_data.access_secret().clone(),
    );

    // Redirect request
    let response = web_hook_data
        .client()
        .post(target_url)
        .headers(target_headers)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to send request: {}", e);
            actix_web::error::ErrorBadRequest(e)
        })?;

    // Parse reqwest response
    let converted_response = ReqwestToActixConverter::convert_response(response).await?;

    debug!("Return response with code {}", converted_response.status());
    Ok(converted_response)
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct WebHookData {
    client: ClientWithMiddleware,
    #[getset(skip)]
    target_host: Url,
    #[getset(skip)]
    allowed_paths: RegexSet,
    access_id: HeaderValue,
    access_secret: HeaderValue,
}

impl WebHookData {
    pub fn new(
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: Vec<String>,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    ) -> Result<Self> {
        let allowed_paths = RegexSet::new(allowed_paths)?;

        let access_id = HeaderValue::from_str(access_id.expose_secret())
            .map_err(|_| Error::custom("Failed to map access id to header value"))?;
        let access_secret = HeaderValue::from_str(access_secret.expose_secret())
            .map_err(|_| Error::custom("Failed to map access secret to header value"))?;
        Ok(Self {
            client,
            target_host,
            allowed_paths,
            access_id,
            access_secret,
        })
    }

    pub fn get_target_url(&self, path: &str) -> Result<Url> {
        self.target_host
            .join(path)
            .map_err(|e| Error::custom(format!("Failed to join URL: {}", e)))
    }

    pub fn is_allowed_path(&self, path: &str) -> bool {
        self.allowed_paths.is_match(path)
    }
}

pub struct Server {
    host: String,
    port: u16,
}

impl Server {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    pub async fn run_until_stopped(&self, web_hook_data: WebHookData) -> Result<()> {
        info!(
            "Starting server on {}:{} with allowed paths {:#?}",
            self.host, self.port, web_hook_data.allowed_paths
        );

        let web_hook_data = web::Data::new(web_hook_data);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web_hook_data.clone())
                .route("/health", web::get().to(get_health_status))
                .route("{tail:.*}", web::post().to(handle_web_hook))
        })
        .bind((self.host.clone(), self.port))?;

        server.run().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests_web_hook_data {
    use reqwest::Url;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use secrecy::Secret;

    use crate::server::WebHookData;

    pub struct TestWebHookData {
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: Vec<String>,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    }

    impl Default for TestWebHookData {
        fn default() -> Self {
            Self {
                client: ClientBuilder::new(reqwest::Client::new()).build(),
                target_host: Url::parse("https://example.com").unwrap(),
                allowed_paths: vec!["/test".to_string()],
                access_id: Secret::new("test id".to_string()),
                access_secret: Secret::new("test secret".to_string()),
            }
        }
    }

    impl From<TestWebHookData> for WebHookData {
        fn from(test_web_hook_data: TestWebHookData) -> Self {
            WebHookData::new(
                test_web_hook_data.client,
                test_web_hook_data.target_host,
                test_web_hook_data.allowed_paths,
                test_web_hook_data.access_id,
                test_web_hook_data.access_secret,
            )
            .unwrap()
        }
    }

    #[test]
    fn test_get_target_url() {
        let base_url = Url::parse("https://example.com").unwrap();
        let web_hook_data: WebHookData = TestWebHookData {
            target_host: base_url,
            ..Default::default()
        }
        .into();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_get_target_url_trailing_slash() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let web_hook_data: WebHookData = TestWebHookData {
            target_host: base_url,
            ..Default::default()
        }
        .into();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_is_allowed_path_invalid_empty() {
        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: vec![],
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/test"));
        assert!(!web_hook_data.is_allowed_path("/test/"));
        assert!(!web_hook_data.is_allowed_path("/test/123"));
    }

    #[test]
    fn test_is_allowed_path_no_regex() {
        let paths = vec!["/test/".to_string(), "/data/123".to_string()];

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/api"));
        assert!(web_hook_data.is_allowed_path("/test/access"));
        assert!(!web_hook_data.is_allowed_path("/data"));

        // Valid paths
        assert!(web_hook_data.is_allowed_path("/test/"));
        assert!(web_hook_data.is_allowed_path("/data/123"));
    }

    #[test]
    fn test_is_allowed_path_regex() {
        let paths = vec!["/test/".to_string(), r"/data/\d*/private".to_string()];

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/api"));
        assert!(!web_hook_data.is_allowed_path("/data"));
        assert!(!web_hook_data.is_allowed_path("/data/123/"));
        assert!(!web_hook_data.is_allowed_path("/data/123/test"));
        assert!(!web_hook_data.is_allowed_path("/data/abc/private"));

        // Valid paths
        assert!(web_hook_data.is_allowed_path("/test/"));
        assert!(web_hook_data.is_allowed_path("/data/123/private"));
    }
}
