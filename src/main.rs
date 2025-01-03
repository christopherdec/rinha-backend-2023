use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use persistence::PeopleRepository;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

mod persistence;

time::serde::format_description!(date_format, Date, "[year]-[month]-[day]");

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct Person {
    pub id: Uuid,
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "apelido")]
    pub nick: String,
    #[serde(rename = "nascimento", with = "date_format")]
    pub birth_date: Date,
    pub stack: Option<Vec<String>>,
}

macro_rules! limited_string_type {
    ($type: ident, max_length = $max_length: expr, error_msg = $error_msg: expr) => {
        #[derive(Deserialize)]
        #[serde(try_from = "String")]        
        pub struct $type(String);

        impl TryFrom<String> for $type {
            type Error = &'static str;
        
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.len() > $max_length {
                    Err($error_msg)
                } else {
                    Ok(Self(value))
                }
            }
        }

        impl From<$type> for String {
            fn from(value: $type) -> Self {
                value.0
            }
        }
    };
}

limited_string_type!(PersonName, max_length = 100, error_msg = "Name length must not exceed 100 characters");
limited_string_type!(Nick, max_length = 32, error_msg = "Nick length must not exceed 32 characters");
limited_string_type!(Tech, max_length = 32, error_msg = "Tech length must not exceed 32 characters");

#[derive(Deserialize)]
pub struct NewPerson {
    #[serde(rename = "nome")]
    pub name: PersonName,
    #[serde(rename = "apelido")]
    pub nick: Nick,
    #[serde(rename = "nascimento", with = "date_format")]
    pub birth_date: Date,
    pub stack: Option<Vec<Tech>>,
}

// note: the sqlx Pool is already wrapped by an Arc
type AppState = Arc<PeopleRepository>;

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(9999);

    let database_url = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://admin:postgres@localhost:5432/rinha",
    ));

    println!("Connect to db url {database_url}");

    let repo = PeopleRepository::connect(database_url).await;

    let app_state = Arc::new(repo);

    let app = Router::new()
        .route("/pessoas", get(search_people))
        .route("/pessoas/:id", get(find_person))
        .route("/pessoas", post(create_person))
        .route("/contagem-pessoas", get(count_people))
        .with_state(app_state);

    println!("Server listening on port {port}");

    let listener =
        tokio::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port))
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
) -> impl IntoResponse {
    match repo.insert(new_person).await {
        Ok(person) => {
            Ok((StatusCode::CREATED, 
                [(header::LOCATION, format!("/pessoas/{}", person.id).parse::<HeaderValue>().unwrap())], 
                Json(person)))
        },
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
