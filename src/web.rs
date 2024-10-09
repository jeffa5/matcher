use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tera::{Context, Tera};

use crate::{db::Database, matching::Graph};

#[derive(Clone)]
pub struct AppState {
    pub tera: Tera,
    pub db: Database,
}

pub async fn root(State(state): State<AppState>) -> Html<String> {
    Html(
        state
            .tera
            .render("index.html", &Context::default())
            .unwrap(),
    )
}

#[derive(Debug, Deserialize)]
pub struct User {
    email: String,
    name: Option<String>,
}

pub async fn find_person(State(state): State<AppState>, Form(user): Form<User>) -> Redirect {
    if let Some(user) = state.db.find_person(&user.email) {
        Redirect::to(&format!("/person/{}", user.id))
    } else if user.name.is_some() {
        state
            .db
            .add_person(user.name.as_ref().unwrap(), &user.email);
        let user = state.db.find_person(&user.email).unwrap();
        Redirect::to(&format!("/person/{}", user.id))
    } else {
        Redirect::to(&format!("/person?email={}", user.email))
    }
}

pub async fn new_person(State(state): State<AppState>, Query(user): Query<User>) -> Html<String> {
    let mut context = Context::new();
    context.insert("email", &user.email);
    context.insert("name", &user.name);
    Html(state.tera.render("add_person.html", &context).unwrap())
}

pub async fn view_person(State(state): State<AppState>, Path(person_id): Path<u32>) -> Response {
    if let Some(user) = state.db.get_person(person_id) {
        let mut matches = state.db.matches_for(person_id);
        matches.sort_by_key(|m| m.0);
        matches.reverse();
        let mut context = Context::new();
        context.insert("id", &user.id);
        context.insert("name", &user.name);
        context.insert("email", &user.email);
        context.insert("waiting", &user.waiting);
        context.insert("matches", &matches);
        Html(state.tera.render("person.html", &context).unwrap()).into_response()
    } else {
        Redirect::to("/person").into_response()
    }
}

pub async fn all_people(State(state): State<AppState>) -> Html<String> {
    let mut context = Context::new();
    let people = state.db.all_people();
    context.insert("people", &people);
    Html(state.tera.render("people.html", &context).unwrap())
}

pub async fn matches(State(state): State<AppState>) -> Html<String> {
    let mut context = Context::new();
    let match_meta = state.db.latest_match_meta();
    let matches = state.db.latest_matches();
    context.insert("match_meta", &match_meta);
    context.insert("matches", &matches);
    Html(state.tera.render("matches.html", &context).unwrap())
}

pub async fn trigger_matching(State(state): State<AppState>) -> Redirect {
    let mut g = Graph::default();

    let mut waiter_index_mapping = HashMap::new();
    let mut index_waiter_mapping = HashMap::new();
    let waiters = state.db.waiters();
    for waiter in &waiters {
        let index = g.add_node(*waiter);
        waiter_index_mapping.insert(*waiter, index);
        index_waiter_mapping.insert(index, *waiter);
    }

    let edges = state.db.edges_for(waiters);
    for (id1, id2, weight) in edges {
        g.add_edge(
            waiter_index_mapping[&id1],
            waiter_index_mapping[&id2],
            weight,
        )
    }

    let matching = g.matching();

    let generation = state.db.add_matching_generation();

    for (p1, p2) in matching {
        state.db.add_matching(
            index_waiter_mapping[&p1],
            p2.map(|p2| index_waiter_mapping[&p2]),
            generation,
        );
    }

    Redirect::to("/matches")
}

pub async fn add_waiter(State(state): State<AppState>, Path(person_id): Path<u32>) -> Redirect {
    state.db.add_waiter(person_id);
    Redirect::to(&format!("/person/{}", person_id))
}
