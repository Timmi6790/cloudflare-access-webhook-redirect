use crate::converter::{ActixToReqwestConverter, ReqwestToActixConverter};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::RegexSet;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;
use secrecy::{ExposeSecret, Secret};

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
    target_headers.append("CF-Access-Client-Id", web_hook_data.access_id.clone());
    target_headers.append(
        "CF-Access-Client-Secret",
        web_hook_data.access_secret.clone(),
    );

    // Redirect request
    let response = web_hook_data
        .client
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

pub struct WebHookData {
    client: ClientWithMiddleware,
    target_host: Url,
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

    pub fn client(&self) -> &ClientWithMiddleware {
        &self.client
    }

    pub fn target_host(&self) -> &Url {
        &self.target_host
    }

    pub fn get_target_url(&self, path: &str) -> Result<Url> {
        self.target_host
            .join(path)
            .map_err(|e| Error::custom(format!("Failed to join URL: {}", e)))
    }

    pub fn is_allowed_path(&self, path: &str) -> bool {
        self.allowed_paths.is_match(path)
    }

    pub fn access_id(&self) -> &HeaderValue {
        &self.access_id
    }

    pub fn access_secret(&self) -> &HeaderValue {
        &self.access_secret
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
    use crate::server::WebHookData;
    use reqwest::Url;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use secrecy::{ExposeSecret, Secret};

    fn get_client() -> ClientWithMiddleware {
        ClientBuilder::new(reqwest::Client::new()).build()
    }

    fn get_target_host() -> Url {
        Url::parse("https://example.com").unwrap()
    }

    fn get_allowed_paths() -> Vec<String> {
        vec!["/test".to_string()]
    }

    fn get_access_id() -> Secret<String> {
        Secret::new("test id".to_string())
    }

    fn get_access_secret() -> Secret<String> {
        Secret::new("test secret".to_string())
    }

    #[test]
    fn test_get_target_url() {
        let base_url = Url::parse("https://example.com").unwrap();
        let web_hook_data = WebHookData::new(
            get_client(),
            base_url,
            get_allowed_paths(),
            get_access_id(),
            get_access_secret(),
        )
        .unwrap();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_get_target_url_trailing_slash() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let web_hook_data = WebHookData::new(
            get_client(),
            base_url,
            get_allowed_paths(),
            get_access_id(),
            get_access_secret(),
        )
        .unwrap();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_is_allowed_path_invalid_empty() {
        let web_hook_data = WebHookData::new(
            get_client(),
            get_target_host(),
            vec![],
            get_access_id(),
            get_access_secret(),
        )
        .unwrap();

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

        let web_hook_data = WebHookData::new(
            get_client(),
            get_target_host(),
            paths,
            get_access_id(),
            get_access_secret(),
        )
        .unwrap();

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

        let web_hook_data = WebHookData::new(
            get_client(),
            get_target_host(),
            paths,
            get_access_id(),
            get_access_secret(),
        )
        .unwrap();

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

    #[test]
    fn test_access_id() {
        let expected_secret = Secret::new("cloudflare access id".to_string());

        let web_hook_data = WebHookData::new(
            get_client(),
            get_target_host(),
            get_allowed_paths(),
            expected_secret.clone(),
            get_access_secret(),
        )
        .unwrap();

        let secret = web_hook_data.access_id();
        assert_eq!(
            secret.to_str().unwrap(),
            expected_secret.expose_secret().as_str()
        );
    }

    #[test]
    fn test_access_secret() {
        let expected_secret = Secret::new("cloudflare access secret".to_string());

        let web_hook_data = WebHookData::new(
            get_client(),
            get_target_host(),
            get_allowed_paths(),
            get_access_id(),
            expected_secret.clone(),
        )
        .unwrap();

        let secret = web_hook_data.access_secret();
        assert_eq!(
            secret.to_str().unwrap(),
            expected_secret.expose_secret().as_str()
        );
    }
}
