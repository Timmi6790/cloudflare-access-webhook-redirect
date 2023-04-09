use crate::converter::{ActixToReqwestConverter, ReqwestToActixConverter};
use crate::data::WebHookData;
use actix_web::http::Method;
use actix_web::web::Query;
use actix_web::{web, HttpRequest, HttpResponse};
use reqwest::{Body, Url};
use reqwest_middleware::{ClientWithMiddleware, RequestBuilder};
use std::collections::HashMap;

pub fn get_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("{tail:.*}")
            .route(web::get().to(post_redirect))
            .route(web::post().to(post_redirect))
            .route(web::put().to(post_redirect))
            .route(web::patch().to(post_redirect))
            .route(web::delete().to(post_redirect)),
    );
}

async fn post_redirect(
    mut payload: web::Payload,
    request: HttpRequest,
    path: web::Path<String>,
    web_hook_data: web::Data<WebHookData>,
) -> core::result::Result<HttpResponse, actix_web::Error> {
    // Only allow specific paths
    info!("Received {} request for path: {}", request.method(), path);
    if !web_hook_data.is_allowed_path(&path, request.method()) {
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
    let mut target_headers: reqwest::header::HeaderMap =
        ActixToReqwestConverter::convert_headers(request.headers(), 2);

    // Add Cloudflare Access headers
    target_headers.append("CF-Access-Client-Id", web_hook_data.access_id().clone());
    target_headers.append(
        "CF-Access-Client-Secret",
        web_hook_data.access_secret().clone(),
    );

    // Query params
    let params = Query::<HashMap<String, String>>::from_query(request.query_string())?;

    // Redirect request
    let response = ReqwestBuilder::new(
        web_hook_data.client(),
        target_url,
        body,
        target_headers,
        params.0,
        request.method(),
    )
    .build()
    .map_err(|e| {
        error!("Failed to build request: {}", e);
        actix_web::error::ErrorBadRequest(e)
    })?
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

struct ReqwestBuilder<'a> {
    client: &'a ClientWithMiddleware,
    url: Url,
    body: Body,
    headers: reqwest::header::HeaderMap,
    params: HashMap<String, String>,

    method: &'a Method,
    include_body: bool,
    include_params: bool,
}

impl<'a> ReqwestBuilder<'a> {
    pub fn new(
        client: &'a ClientWithMiddleware,
        url: Url,
        body: Body,
        headers: reqwest::header::HeaderMap,
        params: HashMap<String, String>,
        method: &'a Method,
    ) -> ReqwestBuilder<'a> {
        ReqwestBuilder {
            client,
            url,
            body,
            headers,
            method,
            params,
            include_body: false,
            include_params: false,
        }
    }

    fn include_body(&mut self) {
        self.include_body = true;
    }

    fn include_params(&mut self) {
        self.include_params = true;
    }

    pub fn build(mut self) -> crate::Result<RequestBuilder> {
        let mut request = match *self.method {
            Method::GET => {
                self.include_params();
                Ok(self.client.get(self.url))
            }
            Method::POST => {
                self.include_body();
                self.include_params();
                Ok(self.client.post(self.url))
            }
            Method::PUT => {
                self.include_body();
                self.include_params();
                Ok(self.client.put(self.url))
            }
            Method::PATCH => {
                self.include_body();
                self.include_params();
                Ok(self.client.patch(self.url))
            }
            Method::DELETE => {
                self.include_params();
                Ok(self.client.delete(self.url))
            }
            _ => Err(crate::Error::invalid_route(self.method)),
        }?;

        // Headers are always required for Cloudflare Access
        request = request.headers(self.headers);

        if self.include_body {
            request = request.body(self.body);
        }

        if self.include_params {
            request = request.query(&self.params);
        }

        Ok(request)
    }
}
