use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router
};
use serde::{Deserialize, Serialize};
use time::{macros::date, Date};
use tokio::sync::RwLock;
use uuid::Uuid;

// a better option would be to use the iso8601 module
time::serde::format_description!(date_format, Date, "[year]-[month]-[day]");

// todo: declare struct and fields as pub if necessary
#[derive(Clone, Serialize)]
struct Person {
    id: Uuid,
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "apelido")]
    nick: String,
    #[serde(rename = "nascimento", with = "date_format")]
    birth_date: Date,
    stack: Option<Vec<String>>
}

#[derive(Deserialize)]
struct NewPerson {
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "apelido")]
    nick: String,
    #[serde(rename = "nascimento", with = "date_format")]
    birth_date: Date,
    stack: Option<Vec<String>>
}

type AppState = Arc<RwLock<HashMap<Uuid, Person>>>;

#[tokio::main]
async fn main() {
    let mut people: HashMap<Uuid, Person> = HashMap::new();

    let person1 = Person { 
        id: Uuid::now_v7(),
        name: String::from("John Smith"), 
        nick: String::from("jsmith"), 
        birth_date: date!(2000 - 01 - 01), 
        stack: Some(vec!["Rust".to_string(), "Java".to_string(), "Python".to_string()])
        // stack: None
    };

    println!("person1.id={}", person1.id);

    // the insert method is a syntatic sugar for HashMap::insert(&mut people, person1.id, person1)
    people.insert(person1.id, person1);

    let app_state = Arc::new(RwLock::new(people));

    let app = Router::new()
        .route("/pessoas", get(search_people))
        .route("/pessoas/:id", get(find_person))
        .route("/pessoas", post(create_person))
        .route("/contagem-pessoas", get(count_people))
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();      
}

async fn search_people(
    State(_people): State<AppState>
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn find_person(
    State(people): State<AppState>,
    Path(person_id): Path<Uuid>
) -> impl IntoResponse {
// ) -> Result<Json<Person>, StatusCode> {
    match people.read().await.get(dbg!(&person_id)) {
        Some(person) => Ok(Json(person.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_person(
    State(people): State<AppState>,
    Json(new_person): Json<NewPerson>
) -> Result<(StatusCode, Json<Person>), StatusCode> {

    if new_person.name.len() > 100
        || new_person.nick.len() > 32
        || new_person.stack
            .as_ref()
            .is_some_and(|s| s.iter().any(|item: &String| item.len() > 32)) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let person = Person {
        id: Uuid::now_v7(),
        name: new_person.name,
        nick: new_person.nick,
        birth_date: new_person.birth_date,
        stack: new_person.stack
    };
    people.write().await.insert(person.id, person.clone());
    Ok((StatusCode::CREATED, Json(person)))
}

async fn count_people(
    State(people): State<AppState>
) -> impl IntoResponse {
    let count = people.read().await.len();
    (StatusCode::OK, Json(count))
}
