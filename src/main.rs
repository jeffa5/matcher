use axum::routing::{get, post};
use tera::Tera;
use tokio::join;

use crate::web::AppState;

mod db;
mod matching;
mod web;

#[tokio::main]
async fn main() {
    let tera = Tera::new("templates/*").unwrap();

    let db = db::Database::init();

    let state = AppState { tera, db };

    let app = axum::Router::new()
        .route("/", axum::routing::get(web::root))
        .route(
            "/person/:person_id",
            get(web::view_person).post(web::toggle_waiter),
        )
        .route("/people", get(web::all_people))
        .route("/matches", get(web::matches))
        .route("/matches/:generation", get(web::matches_generation))
        .route("/sign_in", get(web::sign_in).post(web::do_sign_in))
        .route("/sign_up", get(web::sign_up).post(web::do_sign_up))
        .fallback(web::fallback)
        .with_state(state.clone());

    let ops_app = axum::Router::new()
        .route("/matches", post(web::trigger_matching))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let ops_listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Serving public on http://0.0.0.0:3000");
    println!("Serving private on http://0.0.0.0:3001");
    let public = axum::serve(listener, app);
    let private = axum::serve(ops_listener, ops_app);
    let (a, b) = join![public, private];
    a.unwrap();
    b.unwrap();
}
