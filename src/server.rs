use crate::data::WebHookData;
use crate::routes::{health_check, redirect};
use crate::Result;
use actix_web::{web, App, HttpServer};
use derive_new::new;

#[derive(new)]
pub struct Server {
    host: String,
    port: u16,
}

impl Server {
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
                .configure(health_check::get_config)
                .configure(redirect::get_config)
        })
        .bind((self.host.clone(), self.port))?;

        server.run().await?;

        Ok(())
    }
}
