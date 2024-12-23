use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use persistence::PeopleRepository;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

mod persistence;

// a better option would be to use the iso8601 module
time::serde::format_description!(date_format, Date, "[year]-[month]-[day]");

// todo: declare struct and fields as pub if necessary
#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct Person {
    id: Uuid,
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "apelido")]
    nick: String,
    #[serde(rename = "nascimento", with = "date_format")]
    birth_date: Date,
    stack: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Nick(String);

impl TryFrom<String> for Nick {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > 32 {
            Err("Nick length must not exceed 32 characters")
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct PersonName(String);

impl TryFrom<String> for PersonName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > 100 {
            Err("Name length must not exceed 100 characters")
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Tech(String);

impl TryFrom<String> for Tech {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > 32 {
            Err("Tech length must not exceed 32 characters")
        } else {
            Ok(Self(value))
        }
    }
}

impl From<Tech> for String {
    fn from(value: Tech) -> Self {
        value.0
    }
}

#[derive(Deserialize)]
pub struct NewPerson {
    #[serde(rename = "nome")]
    name: PersonName,
    #[serde(rename = "apelido")]
    nick: Nick,
    #[serde(rename = "nascimento", with = "date_format")]
    birth_date: Date,
    stack: Option<Vec<Tech>>,
}

// note: the sqlx Pool is already wrapped by an Arc
type AppState = Arc<PeopleRepository>;

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(3000);

    let database_url = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://admin:postgres@localhost:5432/rinha",
    ));

    let repo = PeopleRepository::connect(database_url).await;

    let app_state = Arc::new(repo);

    let app = Router::new()
        .route("/pessoas", get(search_people))
        .route("/pessoas/:id", get(find_person))
        .route("/pessoas", post(create_person))
        .route("/contagem-pessoas", get(count_people))
        .with_state(app_state);

    let listener =
        tokio::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct PersonSearchParams {
    #[serde(rename = "t")]
    query: String,
}

async fn search_people(
    State(repo): State<AppState>,
    Query(PersonSearchParams { query }): Query<PersonSearchParams>,
) -> impl IntoResponse {
    match repo.search(query).await {
        Ok(people) => Ok(Json(people)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn find_person(
    State(repo): State<AppState>,
    Path(person_id): Path<Uuid>,
) -> impl IntoResponse {
    match repo.find_by_id(person_id).await {
        Ok(Some(person)) => Ok(Json(person)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn create_person(
    State(repo): State<AppState>,
    Json(new_person): Json<NewPerson>,
) -> Result<(StatusCode, Json<Person>), StatusCode> {
    match repo.insert(new_person).await {
        Ok(person) => Ok((StatusCode::CREATED, Json(person))),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn count_people(State(repo): State<AppState>) -> impl IntoResponse {
    match repo.count().await {
        Ok(count) => Ok(Json(count)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
