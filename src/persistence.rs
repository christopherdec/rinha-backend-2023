use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::{NewPerson, Person};

pub struct PeopleRepository {
    pool: PgPool,
}

impl PeopleRepository {
    pub async fn connect(url: String) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();

        PeopleRepository { pool }
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Person>, sqlx::Error> {
        sqlx::query_as!(
            Person,
            "
            SELECT id, name, nick, birth_date, stack 
            FROM people
            WHERE id = $1
            ",
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert(&self, new_person: NewPerson) -> Result<Person, sqlx::Error> {
        let stack = new_person.stack
            .map(|stack| stack.into_iter().map(String::from).collect::<Vec<String>>());

        sqlx::query_as!(
            Person,
            "
            INSERT INTO people (id, name, nick, birth_date, stack)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, nick, birth_date, stack
            ",
            Uuid::now_v7(),
            new_person.name.0,
            new_person.nick.0,
            new_person.birth_date,
            stack.as_deref()
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn search(&self, query: String) -> Result<Vec<Person>, sqlx::Error> {
        sqlx::query_as!(
            Person,
            "
            SELECT id, name, nick, birth_date, stack 
            FROM people
            WHERE search ILIKE $1
            LIMIT 50
            ",
            format!("%{query}%")
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "
            SELECT count(*)
            FROM people
            ",
        )
        .fetch_one(&self.pool)
        .await
    }
}
