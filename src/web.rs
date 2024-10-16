use std::collections::HashMap;

use axum::{
    extract::{FromRef, FromRequestParts, Path, Query, State},
    http::{header::SET_COOKIE, request::Parts},
    response::{AppendHeaders, Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use serde::Deserialize;
use tera::{Context, Tera};

use crate::{
    db::{Database, SignInError},
    matching::Graph,
};

pub fn session_id_cookie(session_id: &str) -> String {
    Cookie::new("session_id", session_id).to_string()
}

// An extractor that performs authorization.
pub struct Authorized {
    session_id: String,
    person_id: u32,
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for Authorized
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_request_parts(parts, state).await.unwrap();

        let state = AppState::from_ref(state);

        let Some(session_id) = cookies.get("session_id").and_then(|c| {
            let v = c.value();
            if v.is_empty() {
                None
            } else {
                Some(v.to_owned())
            }
        }) else {
            let mut context = Context::default();
            context.insert("error", "It seems like you aren't signed in. Please either sign up to this Matcher, or sign in.");
            let error_page = Html(state.tera.render("error.html", &context).unwrap());
            return Err(error_page.into_response());
        };

        let now = chrono::offset::Utc::now().timestamp();
        match state.db.get_session(&session_id, now) {
            Some(person_id) => Ok(Self {
                session_id,
                person_id,
            }),
            _ => {
                let mut context = Context::default();
                context.insert(
                    "error",
                    "Failed to find your session, please try logging in again.",
                );
                let error_page = Html(state.tera.render("error.html", &context).unwrap());
                return Err((
                    AppendHeaders([(SET_COOKIE, session_id_cookie(""))]),
                    error_page.into_response(),
                )
                    .into_response());
            }
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub tera: Tera,
    pub db: Database,
}

pub async fn root(State(state): State<AppState>, authorized: Option<Authorized>) -> Html<String> {
    let mut context = Context::default();
    if let Some(authorized) = authorized {
        context.insert("authorized_person_id", &authorized.person_id);
    }
    Html(state.tera.render("index.html", &context).unwrap())
}

pub async fn sign_in(State(state): State<AppState>) -> Html<String> {
    Html(
        state
            .tera
            .render("sign_in.html", &Context::default())
            .unwrap(),
    )
}

#[derive(Clone, Deserialize)]
pub struct SignIn {
    email: String,
    password: String,
}

pub async fn do_sign_in(State(state): State<AppState>, Form(user): Form<SignIn>) -> Response {
    match state.db.sign_in_session(&user.email, &user.password) {
        Ok(session_id) => {
            let headers = AppendHeaders([(SET_COOKIE, session_id_cookie(&session_id))]);
            (headers, Redirect::to("/")).into_response()
        }
        Err(SignInError::UnknownUser) => {
            Redirect::to(&format!("/sign_up?email={}", user.email)).into_response()
        }
        Err(SignInError::InvalidPassword) => {
            let mut context = Context::default();
            context.insert(
                "error",
                "Failed to sign you in, please check your password.",
            );
            let error_page = Html(state.tera.render("sign_in.html", &context).unwrap());
            error_page.into_response()
        }
    }
}

pub async fn sign_out(State(state): State<AppState>, authorized: Authorized) -> Redirect {
    state.db.sign_out_session(&authorized.session_id);
    Redirect::to("/")
}

#[derive(Debug, Deserialize)]
pub struct SignUp {
    email: String,
    password: String,
    name: String,
}

pub async fn do_sign_up(State(state): State<AppState>, Form(sign_up): Form<SignUp>) -> Response {
    let (user_id, session_id) =
        state
            .db
            .sign_up_session(&sign_up.name, &sign_up.email, &sign_up.password);
    (
        AppendHeaders([(SET_COOKIE, session_id_cookie(&session_id))]),
        Redirect::to(&format!("/person/{}", user_id)),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct SignUpQuery {
    email: Option<String>,
}

pub async fn sign_up(
    State(state): State<AppState>,
    Query(sign_up_query): Query<SignUpQuery>,
) -> Html<String> {
    let mut context = Context::default();
    context.insert("email", &sign_up_query.email);
    Html(state.tera.render("sign_up.html", &context).unwrap())
}

pub async fn view_person(
    State(state): State<AppState>,
    authorized: Authorized,
    Path(person_id): Path<u32>,
) -> Response {
    if let Some((user, mut matches)) = state.db.get_person_and_matches(person_id) {
        matches.sort_by_key(|m| m.0);
        matches.reverse();
        let mut context = Context::new();
        context.insert("id", &user.id);
        context.insert("name", &user.name);
        context.insert("email", &user.email);
        context.insert("waiting", &user.waiting);
        context.insert("matches", &matches);
        context.insert("authorized_person_id", &authorized.person_id);
        Html(state.tera.render("person.html", &context).unwrap()).into_response()
    } else {
        Redirect::to("/person").into_response()
    }
}

pub async fn all_people(State(state): State<AppState>, authorized: Authorized) -> Html<String> {
    let mut context = Context::new();
    context.insert("authorized_person_id", &authorized.person_id);
    let people = state.db.all_people();
    context.insert("people", &people);
    Html(state.tera.render("people.html", &context).unwrap())
}

pub async fn matches(State(state): State<AppState>, authorized: Authorized) -> Html<String> {
    let mut context = Context::new();
    context.insert("authorized_person_id", &authorized.person_id);
    if let Some((match_meta, matches)) = state.db.latest_matches() {
        context.insert("match_meta", &match_meta);
        context.insert("matches", &matches);
    }
    Html(state.tera.render("matches.html", &context).unwrap())
}

pub async fn matches_generation(
    State(state): State<AppState>,
    authorized: Authorized,
    Path(generation): Path<u32>,
) -> Html<String> {
    let mut context = Context::new();
    context.insert("authorized_person_id", &authorized.person_id);
    if let Some((match_meta, matches)) = state.db.matches_at(generation) {
        context.insert("match_meta", &match_meta);
        context.insert("matches", &matches);
    }
    Html(state.tera.render("matches.html", &context).unwrap())
}

pub async fn trigger_matching(State(state): State<AppState>) -> Redirect {
    let mut g = Graph::default();

    let mut waiter_index_mapping = HashMap::new();
    let mut index_waiter_mapping = HashMap::new();
    let waiters = state.db.waiters();
    if waiters.is_empty() {
        return Redirect::to("/matches");
    }

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

pub async fn toggle_waiter(
    State(state): State<AppState>,
    authorized: Authorized,
    Path(person_id): Path<u32>,
) -> Redirect {
    if authorized.person_id == person_id {
        state.db.toggle_waiter(person_id);
    }
    Redirect::to(&format!("/person/{}", person_id))
}

pub async fn fallback() -> Redirect {
    Redirect::to("/")
}
