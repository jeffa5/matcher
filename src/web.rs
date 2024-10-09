use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tera::{Context, Tera};

use crate::{db::Database, matching::Graph};

pub async fn root() -> Html<String> {
    Html(include_str!("../templates/index.html").to_owned())
}

#[derive(Debug, Deserialize)]
pub struct User {
    email: String,
    name: Option<String>,
}

pub async fn find_person(
    State((_tera, db)): State<(Tera, Database)>,
    Form(user): Form<User>,
) -> Redirect {
    if let Some(user) = db.find_person(&user.email) {
        Redirect::to(&format!("/person/{}", user.id))
    } else if user.name.is_some() {
        db.add_person(user.name.as_ref().unwrap(), &user.email);
        let user = db.find_person(&user.email).unwrap();
        Redirect::to(&format!("/person/{}", user.id))
    } else {
        Redirect::to(&format!("/person?email={}", user.email))
    }
}

pub async fn new_person(
    State((tera, _db)): State<(Tera, Database)>,
    Query(user): Query<User>,
) -> Html<String> {
    let mut context = Context::new();
    context.insert("email", &user.email);
    context.insert("name", &user.name);
    Html(tera.render("add_person.html", &context).unwrap())
}

pub async fn view_person(
    State((tera, db)): State<(Tera, Database)>,
    Path(person_id): Path<u32>,
) -> Response {
    if let Some(user) = db.get_person(person_id) {
        let mut matches = db.matches_for(person_id);
        matches.sort_by_key(|m| m.0);
        matches.reverse();
        let mut context = Context::new();
        context.insert("id", &user.id);
        context.insert("name", &user.name);
        context.insert("email", &user.email);
        context.insert("waiting", &user.waiting);
        context.insert("matches", &matches);
        Html(tera.render("person.html", &context).unwrap()).into_response()
    } else {
        Redirect::to("/person").into_response()
    }
}

pub async fn all_people(State((tera, db)): State<(Tera, Database)>) -> Html<String> {
    let mut context = Context::new();
    let people = db.all_people();
    context.insert("people", &people);
    Html(tera.render("people.html", &context).unwrap())
}

pub async fn matches(State((tera, db)): State<(Tera, Database)>) -> Html<String> {
    let mut context = Context::new();
    let match_generation = db.latest_generation();
    let matches = db.latest_matches();
    context.insert("match_generation", &match_generation);
    context.insert("matches", &matches);
    Html(tera.render("matches.html", &context).unwrap())
}

pub async fn trigger_matching(State((tera, db)): State<(Tera, Database)>) -> Redirect {
    println!("Start matching");

    let mut g = Graph::default();

    let mut waiter_index_mapping = HashMap::new();
    let mut index_waiter_mapping = HashMap::new();
    let waiters = db.waiters();
    dbg!(&waiters);
    for waiter in &waiters {
        let index = g.add_node(*waiter);
        waiter_index_mapping.insert(*waiter, index);
        index_waiter_mapping.insert(index, *waiter);
    }

    let edges = db.edges_for(waiters);
    dbg!(&edges);
    for (id1, id2, weight) in edges {
        g.add_edge(
            waiter_index_mapping[&id1],
            waiter_index_mapping[&id2],
            weight,
        )
    }
    dbg!(&g);

    let matching = g.matching();
    dbg!(&matching);

    let generation = db.add_matching_generation();
    dbg!(generation);

    for (p1, p2) in matching {
        db.add_matching(
            index_waiter_mapping[&p1],
            p2.map(|p2| index_waiter_mapping[&p2]),
            generation,
        );
    }

    Redirect::to("/matches")
}

pub async fn add_waiter(
    State((_tera, db)): State<(Tera, Database)>,
    Path(person_id): Path<u32>,
) -> Redirect {
    db.add_waiter(person_id);
    Redirect::to(&format!("/person/{}", person_id))
}
