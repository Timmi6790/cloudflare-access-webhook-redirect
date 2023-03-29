use crate::converter::{ActixToReqwestConverter, ReqwestToActixConverter};
use crate::data::WebHookData;
use actix_web::{web, HttpRequest, HttpResponse};

pub fn get_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("{tail:.*}").route(web::post().to(post_redirect)));
}

// TODO: Add query support
// TODO: Add more method support?
async fn post_redirect(
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
    let mut target_headers: reqwest::header::HeaderMap =
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
