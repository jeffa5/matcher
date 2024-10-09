use std::{fs::File, path::PathBuf};

use axum::routing::{get, post};
use tera::Tera;

mod db;
mod matching;
mod web;

struct Storage {
    path: PathBuf,
}

#[tokio::main]
async fn main() {
    let tera = Tera::new("templates/*").unwrap();

    let db = db::Database::init();

    let app = axum::Router::new()
        .route("/", axum::routing::get(web::root))
        .route("/person", get(web::new_person).post(web::find_person))
        .route(
            "/person/:person_id",
            get(web::view_person).post(web::add_waiter),
        )
        .route("/people", get(web::all_people))
        .route("/matches", get(web::matches))
        .route("/matches", post(web::trigger_matching))
        .with_state((tera, db));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Serving on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
