use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub enum SignInError {
    UnknownUser,
    InvalidPassword,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: u32,
    pub email: String,
    pub name: String,
    pub waiting: bool,
}

#[derive(Debug, Serialize)]
pub struct Match {
    pub person1: Person,
    pub person2: Option<Person>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchMeta {
    pub generation: u32,
    pub time: u64,
}

const CREATE_TABLE_PEOPLE: &str = "CREATE TABLE IF NOT EXISTS people (
    id integer primary key,
    email text not null unique,
    name text not null,
    waiting boolean not null
)";

const CREATE_TABLE_MATCHES: &str = "CREATE TABLE IF NOT EXISTS matches (
    generation integer not null,
    person1 text not null,
    person2 text,
    foreign key(generation) references generations(id),
    foreign key(person1) references people(id),
    foreign key(person2) references people(id)
)";

const CREATE_TABLE_GENERATIONS: &str = "CREATE TABLE IF NOT EXISTS generations (
    id integer primary key,
    time integer
)";

const CREATE_TABLE_EDGES: &str = "CREATE TABLE IF NOT EXISTS edges (
    person1 integer not null,
    person2 integer not null,
    weight integer not null,
    primary key(person1, person2),
    foreign key(person1) references people(id),
    foreign key(person2) references people(id)
)";

const CREATE_TABLE_AUTH: &str = "CREATE TABLE IF NOT EXISTS auth (
    person INTEGER NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    FOREIGN KEY(person) REFERENCES people(id)
)";

const CREATE_TABLE_SESSIONS: &str = "CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    person INTEGER NOT NULL UNIQUE,
    last_seen INTEGER NOT NULL,
    FOREIGN KEY(person) REFERENCES people(id)
)";

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<rusqlite::Connection>>,
}

impl Database {
    pub fn init() -> Database {
        let conn = Connection::open("matcher.sqlite").unwrap();
        let s = Database {
            connection: Arc::new(Mutex::new(conn)),
        };
        s.migrate();
        s
    }

    fn migrate(&self) {
        let conn = self.connection.lock().unwrap();
        let creations = [
            CREATE_TABLE_PEOPLE,
            CREATE_TABLE_GENERATIONS,
            CREATE_TABLE_MATCHES,
            CREATE_TABLE_EDGES,
            CREATE_TABLE_AUTH,
            CREATE_TABLE_SESSIONS,
        ];
        for creation in creations {
            conn.execute(creation, []).unwrap();
        }
    }

    pub fn get_person(&self, id: u32) -> Option<Person> {
        self.connection
            .lock()
            .unwrap()
            .query_row(
                "select p.id, p.email, p.name, p.waiting from people p
                 where p.id = ?1",
                [id],
                |row| {
                    Ok(Person {
                        id: row.get(0).unwrap(),
                        email: row.get(1).unwrap(),
                        name: row.get(2).unwrap(),
                        waiting: row.get(3).unwrap(),
                    })
                },
            )
            .ok()
    }

    pub fn sign_up_session(&self, name: &str, email: &str, password: &str) -> (u32, String) {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let conn = self.connection.lock().unwrap();
        let id: u32 = conn
            .query_row(
                "insert into people (email, name, waiting) values (?1, ?2, FALSE) RETURNING id",
                [email, name],
                |row| row.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO auth (person, password_hash) values (?1, ?2)",
            params![id, password_hash],
        )
        .unwrap();
        let session_id = uuid::Uuid::new_v4().to_string();
        let time = chrono::offset::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sessions (id, person, last_seen) VALUES (?1, ?2, ?3)",
            params![session_id, id, time],
        )
        .unwrap();
        (id, session_id)
    }

    pub fn toggle_waiter(&self, person_id: u32) {
        self.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE people SET waiting = (CASE WHEN waiting = FALSE THEN TRUE ELSE FALSE END) WHERE id = ?1",
                [person_id],
            )
            .unwrap();
    }

    pub fn matches_for(&self, person_id: u32) -> Vec<(u32, Person)> {
        let conn = self.connection.lock().unwrap();
        let mut stmnt = conn
            .prepare("select m.generation, p.id, p.name from matches m join people p on m.person2 = p.id WHERE m.person1 = ?1")
            .unwrap();
        let mut rows = stmnt.query([person_id]).unwrap();

        let mut people = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            people.push((
                row.get(0).unwrap(),
                Person {
                    id: row.get(1).unwrap(),
                    email: "".to_owned(),
                    name: row.get(2).unwrap(),
                    waiting: false,
                },
            ));
        }

        let mut stmnt = conn
            .prepare("select m.generation, p.id, p.name from matches m join people p on m.person1 = p.id WHERE m.person2 = ?1")
            .unwrap();
        let mut rows = stmnt.query([person_id]).unwrap();

        while let Some(row) = rows.next().unwrap() {
            people.push((
                row.get(0).unwrap(),
                Person {
                    id: row.get(1).unwrap(),
                    email: "".to_owned(),
                    name: row.get(2).unwrap(),
                    waiting: false,
                },
            ));
        }

        people
    }

    pub fn all_people(&self) -> Vec<Person> {
        let conn = self.connection.lock().unwrap();
        let mut stmnt = conn
            .prepare("select p.id, p.email, p.name, p.waiting from people p")
            .unwrap();
        let mut rows = stmnt.query([]).unwrap();

        let mut people = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            people.push(Person {
                id: row.get(0).unwrap(),
                email: row.get(1).unwrap(),
                name: row.get(2).unwrap(),
                waiting: row.get(3).unwrap(),
            });
        }
        people
    }

    pub fn match_meta_at(&self, generation: u32) -> Option<MatchMeta> {
        self.connection
            .lock()
            .unwrap()
            .query_row(
                "select id, time from generations where id = ?1",
                [generation],
                |r| {
                    Ok(MatchMeta {
                        generation: r.get(0)?,
                        time: r.get(1)?,
                    })
                },
            )
            .ok()
    }

    pub fn latest_match_meta(&self) -> Option<MatchMeta> {
        self.connection
            .lock()
            .unwrap()
            .query_row("select max(id), time from generations", [], |r| {
                Ok(MatchMeta {
                    generation: r.get(0)?,
                    time: r.get(1)?,
                })
            })
            .ok()
    }

    pub fn matches_at(&self, generation: u32) -> Vec<Match> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn
            .prepare("select p1.id, p1.email, p1.name, p1.waiting, p2.id, p2.email, p2.name, p2.waiting from matches m join people p1 on m.person1 = p1.id join people p2 on m.person2 = p2.id where m.generation = ?1")
            .unwrap();
        let mut rows = stmt.query([generation]).unwrap();
        let mut matches = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            matches.push(Match {
                person1: Person {
                    id: row.get(0).unwrap(),
                    email: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    waiting: row.get(3).unwrap(),
                },
                person2: Some(Person {
                    id: row.get(4).unwrap(),
                    email: row.get(5).unwrap(),
                    name: row.get(6).unwrap(),
                    waiting: row.get(7).unwrap(),
                }),
            })
        }
        let mut stmt = conn
            .prepare("select p1.id, p1.email, p1.name, p1.waiting from matches m join people p1 on m.person1 = p1.id where m.generation = ?1 AND m.person2 IS NULL")
            .unwrap();
        let mut rows = stmt.query([generation]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            matches.push(Match {
                person1: Person {
                    id: row.get(0).unwrap(),
                    email: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    waiting: row.get(3).unwrap(),
                },
                person2: None,
            })
        }
        matches
    }

    pub fn latest_matches(&self) -> Vec<Match> {
        let Some(latest_match_meta) = self.latest_match_meta() else {
            return Vec::new();
        };
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn
            .prepare("select p1.id, p1.email, p1.name, p1.waiting, p2.id, p2.email, p2.name, p2.waiting from matches m join people p1 on m.person1 = p1.id join people p2 on m.person2 = p2.id where m.generation = ?1")
            .unwrap();
        let mut rows = stmt.query([latest_match_meta.generation]).unwrap();
        let mut matches = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            matches.push(Match {
                person1: Person {
                    id: row.get(0).unwrap(),
                    email: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    waiting: row.get(3).unwrap(),
                },
                person2: Some(Person {
                    id: row.get(4).unwrap(),
                    email: row.get(5).unwrap(),
                    name: row.get(6).unwrap(),
                    waiting: row.get(7).unwrap(),
                }),
            })
        }
        let mut stmt = conn
            .prepare("select p1.id, p1.email, p1.name, p1.waiting from matches m join people p1 on m.person1 = p1.id where m.generation = ?1 AND m.person2 IS NULL")
            .unwrap();
        let mut rows = stmt.query([latest_match_meta.generation]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            matches.push(Match {
                person1: Person {
                    id: row.get(0).unwrap(),
                    email: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    waiting: row.get(3).unwrap(),
                },
                person2: None,
            })
        }
        matches
    }

    pub fn add_matching(&self, p1id: u32, p2id: Option<u32>, generation: u32) {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO matches (generation, person1, person2) VALUES (?1, ?2, ?3)",
            params![generation, p1id, p2id],
        )
        .unwrap();
        if let Some(p2id) = p2id {
            conn.execute(
            "INSERT INTO edges (person1, person2, weight) VALUES (?1, ?2, 1) ON CONFLICT (person1, person2) DO UPDATE SET weight = weight + 1",
            params![p1id, p2id],
            )
            .unwrap();
            conn.execute(
                "UPDATE people SET waiting = FALSE WHERE id = ?1 OR id = ?2",
                params![p1id, p2id],
            )
            .unwrap();
        } else {
            conn.execute(
                "UPDATE people SET waiting = FALSE WHERE id = ?1",
                params![p1id],
            )
            .unwrap();
        }
    }

    pub fn add_matching_generation(&self) -> u32 {
        let time = chrono::offset::Utc::now().timestamp();
        self.connection
            .lock()
            .unwrap()
            .query_row(
                "insert into generations (id, time) values ((select max(id) + 1 from generations), ?1) returning id",
                [time],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn waiters(&self) -> Vec<u32> {
        let conn = self.connection.lock().unwrap();
        let mut stmnt = conn
            .prepare("select id from people WHERE waiting = TRUE")
            .unwrap();
        let mut rows = stmnt.query([]).unwrap();
        let mut people = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            people.push(row.get(0).unwrap());
        }
        people
    }

    pub fn edges_for(&self, waiters: Vec<u32>) -> Vec<(u32, u32, u32)> {
        let conn = self.connection.lock().unwrap();
        let mut stmnt = conn.prepare("select * from edges e").unwrap();
        // maybe use rarray module
        let mut rows = stmnt.query([]).unwrap();
        let mut edges = Vec::new();
        let waiters = HashSet::<u32>::from_iter(waiters);
        while let Some(row) = rows.next().unwrap() {
            let p1 = row.get(0).unwrap();
            let p2 = row.get(1).unwrap();
            let weight = row.get(2).unwrap();
            if waiters.contains(&p1) && waiters.contains(&p2) {
                edges.push((p1, p2, weight));
            }
        }
        edges
    }

    pub fn get_session(&self, session_id: &str, now: i64) -> Option<u32> {
        let conn = self.connection.lock().unwrap();
        let (person, last_seen) = conn
            .query_row(
                "SELECT person, last_seen FROM sessions WHERE id = ?1",
                [session_id],
                |row| Ok((row.get(0).unwrap(), row.get::<_, i64>(1).unwrap())),
            )
            .ok()?;
        let week_seconds = 60 * 60 * 24 * 7;
        if last_seen - now > week_seconds {
            conn.execute("DELETE FROM sessions WHERE id = ?1", [session_id])
                .unwrap();
            None
        } else {
            let time = chrono::offset::Utc::now().timestamp();
            conn.execute(
                "UPDATE sessions SET last_seen = ?2 WHERE id = ?1",
                params![session_id, time],
            )
            .unwrap();
            Some(person)
        }
    }

    pub fn sign_in_session(&self, email: &str, password: &str) -> Result<String, SignInError> {
        let conn = self.connection.lock().unwrap();
        let expected_password_hash: Result<String, _> = conn.query_row(
            "SELECT password_hash FROM auth JOIN people ON id = person WHERE email = ?1",
            [email],
            |r| r.get(0),
        );
        match expected_password_hash {
            Err(_) => return Err(SignInError::UnknownUser),
            Ok(expected_password_hash) => {
                if Argon2::default()
                    .verify_password(
                        password.as_bytes(),
                        &PasswordHash::new(&expected_password_hash).unwrap(),
                    )
                    .is_err()
                {
                    return Err(SignInError::InvalidPassword);
                }
            }
        }
        let session_id = uuid::Uuid::new_v4().to_string();
        let time = chrono::offset::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sessions (id, person, last_seen) VALUES (?1, (SELECT id FROM people WHERE email = ?2), ?3)",
            params![session_id, email, time],
        ).unwrap();
        Ok(session_id)
    }

    pub fn sign_out_session(&self, session_id: &str) {
        let conn = self.connection.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE id = ?1", [session_id])
            .unwrap();
    }
}
