use crate::data::WebHookData;
use actix_web::{web, App, HttpServer};

use crate::routes::{health_check, post_redirect};
use crate::Result;

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
            self.host,
            self.port,
            web_hook_data.allowed_paths()
        );

        let web_hook_data = web::Data::new(web_hook_data);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web_hook_data.clone())
                .route("/health", web::get().to(health_check))
                .route("{tail:.*}", web::post().to(post_redirect))
        })
        .bind((self.host.clone(), self.port))?;

        server.run().await?;

        Ok(())
    }
}
