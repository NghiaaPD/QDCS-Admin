// use duckdb::{Connection, Result};

// pub fn show_data() -> Result<()> {
//     let conn = Connection::open("data.duckdb")?;

//     let mut stmt = conn.prepare("
//         SELECT 
//             CAST(question_embedding AS JSON) as question_json,
//             CAST(answer_embedding AS JSON) as answer_json 
//         FROM data
//     ")?;

//     let rows = stmt.query_map([], |row| {
//         Ok((
//             row.get::<_, String>(0)?,
//             row.get::<_, String>(1)?,
//         ))
//     })?;

//     println!("\n=== Dữ liệu trong Database ===");
//     let mut count = 0;
//     for (i, row) in rows.enumerate() {
//         if let Ok((question_emb, answer_emb)) = row {
//             println!("\nDòng {}:", i + 1);
//             println!("Question embedding: {}", question_emb);
//             println!("Answer embedding: {}", answer_emb);
//             count += 1;
//         }
//     }
    
//     if count == 0 {
//         println!("Database trống, chưa có dữ liệu nào!");
//     } else {
//         println!("\nTổng số dòng: {}", count);
//     }

//     Ok(())
// }

// fn main() {
//     match show_data() {
//         Ok(_) => println!("\nĐã đọc dữ liệu thành công!"),
//         Err(e) => println!("Lỗi khi đọc dữ liệu: {}", e)
//     }
// }
