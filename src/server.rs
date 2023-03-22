use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use regex::RegexSet;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;
use secrecy::{ExposeSecret, Secret};
use tokio_stream::StreamExt;

use crate::error::Error;
use crate::Result;

fn is_valid_header_name(name: &str) -> bool {
    return match name {
        "host" => false,
        _ => true,
    };
}

// TODO: Just forward the request to the configured URL and don't try to verify anything. The verification should be done purely by the internal serivce
// TODO: Do the conversion in specific converter structs
async fn handle_web_hook(
    mut payload: web::Payload,
    request: HttpRequest,
    path: web::Path<String>,
    web_hook_data: web::Data<WebHookData>,
) -> core::result::Result<HttpResponse, actix_web::Error> {
    // Only allow specific paths
    if !web_hook_data.is_allowed_path(&path) {
        return Ok(HttpResponse::NotFound().finish());
    }

    // Get content
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    // Craft target url
    let target_url = web_hook_data.get_target_url(path.as_str()).map_err(|e| {
        error!("Failed to join URL: {}", e);
        actix_web::error::ErrorBadRequest(e)
    })?;

    // Convert headers to reqwest headers
    let mut target_headers: HeaderMap = HeaderMap::with_capacity(request.headers().capacity() + 2);
    for (key, value) in request.headers().iter() {
        if let Ok(value) = HeaderValue::from_bytes(value.as_bytes()) {
            if is_valid_header_name(key.as_str()) {
                target_headers.append(key, value);
            }
        }
    }

    // Add Cloudflare Access headers
    target_headers.append("CF-Access-Client-Id", web_hook_data.access_id.clone());
    target_headers.append(
        "CF-Access-Client-Secret",
        web_hook_data.access_secret.clone(),
    );

    // Redirect request
    let body = reqwest::Body::from(bytes.freeze());
    let response = web_hook_data
        .client
        .put(target_url)
        .headers(target_headers)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to send request: {}", e);
            actix_web::error::ErrorBadRequest(e)
        })?;

    // Craft response
    println!("Response: {:?}", response);

    let response_code = StatusCode::from_u16(response.status().as_u16()).map_err(|e| {
        error!("Failed to convert response code: {}", e);
        actix_web::error::ErrorBadRequest(e)
    })?;

    println!("Response headers: {:?}", response.headers());

    let response_body = response.text().await.map_err(|e| {
        error!("Failed to read response body: {}", e);
        actix_web::error::ErrorBadRequest(e)
    })?;
    println!("Response content: {:?}", response_body);

    let response = HttpResponse::build(response_code).body(response_body);
    Ok(response)
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
            .map_err(|e| Error::custom("Failed to map access id to header value"))?;
        let access_secret = HeaderValue::from_str(access_secret.expose_secret())
            .map_err(|e| Error::custom("Failed to map access secret to header value"))?;
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
        info!("Starting server on {}:{}", self.host, self.port);

        let web_hook_data = web::Data::new(web_hook_data);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web_hook_data.clone())
                .route("{tail:.*}", web::post().to(handle_web_hook))
        })
        .bind((self.host.clone(), self.port))?;

        server.run().await?;

        Ok(())
    }
}
