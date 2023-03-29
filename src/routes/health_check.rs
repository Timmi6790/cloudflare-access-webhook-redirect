use actix_web::{web, HttpResponse};

pub fn get_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/health").route(web::get().to(HttpResponse::Ok)));
}

#[cfg(test)]
mod tests {
    use crate::routes::health_check::get_config;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_handle_web_hook() {
        let app = test::init_service(App::new().configure(get_config)).await;

        let req = test::TestRequest::get().uri("/health").to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
