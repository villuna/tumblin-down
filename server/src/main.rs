use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .nest_service("/", ServeDir::new("../site").not_found_service(ServeFile::new("../site/index.html")))
        .nest_service("/crate", ServeDir::new("../crate"));

    axum::Server::bind(&"0.0.0.0:2611".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}