use duckdb::{Connection, Result};

pub fn create_database() -> Result<()> {
    let conn = Connection::open("data.duckdb")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS data (
            question_embedding REAL[] NOT NULL,
            answer_embedding REAL[] NOT NULL
        )",
        [],
    )?;

    Ok(())
}
