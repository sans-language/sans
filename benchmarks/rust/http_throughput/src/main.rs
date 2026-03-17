use actix_web::{web, App, HttpResponse, HttpServer};

async fn handler() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"message":"hello","n":42}"#)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::get().to(handler)))
        .bind("127.0.0.1:8765")?
        .run()
        .await
}
