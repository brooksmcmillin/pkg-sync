use anyhow::Result;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS machines (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            api_key_hash TEXT NOT NULL,
            last_seen DATETIME
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS packages (
            id INTEGER PRIMARY KEY,
            machine_id INTEGER NOT NULL REFERENCES machines(id),
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'repo',
            UNIQUE(machine_id, name)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS exclusions (
            id INTEGER PRIMARY KEY,
            machine_id INTEGER NOT NULL REFERENCES machines(id),
            pattern TEXT NOT NULL,
            UNIQUE(machine_id, pattern)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn register_machine(pool: &SqlitePool, name: &str) -> Result<String> {
    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);

    sqlx::query("INSERT INTO machines (name, api_key_hash) VALUES (?, ?)")
        .bind(name)
        .bind(&key_hash)
        .execute(pool)
        .await?;

    Ok(api_key)
}

fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

pub async fn lookup_machine_by_key(
    pool: &SqlitePool,
    api_key: &str,
) -> Result<Option<(i64, String)>> {
    let key_hash = hash_api_key(api_key);
    let row =
        sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM machines WHERE api_key_hash = ?")
            .bind(&key_hash)
            .fetch_optional(pool)
            .await?;

    Ok(row)
}

pub async fn get_machine_by_name(pool: &SqlitePool, name: &str) -> Result<Option<i64>> {
    let row = sqlx::query_scalar::<_, i64>("SELECT id FROM machines WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}
