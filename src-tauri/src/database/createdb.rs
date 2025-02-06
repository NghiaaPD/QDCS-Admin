use duckdb::{Connection, Result};

pub fn create_database() -> Result<()> {
    let conn = Connection::open("data.duckdb")?;

    // Lấy danh sách tất cả các bảng và xóa từng bảng
    conn.execute(
        "SELECT 'DROP TABLE ' || table_name || ';'
         FROM information_schema.tables 
         WHERE table_schema = 'main'",
        [],
    )?;

    // Tạo lại bảng QCData
    conn.execute(
        "CREATE TABLE IF NOT EXISTS data (
            question_embedding REAL[] NOT NULL,
            answer_embedding REAL[] NOT NULL
        )",
        [],
    )?;

    Ok(())
}
