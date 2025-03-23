use duckdb::{Connection, Result};
use serde_json;

pub fn query_db() -> Result<Vec<(Vec<f32>, Vec<f32>)>> {
    let conn = Connection::open("data.duckdb")?;

    let mut stmt = conn.prepare("
        SELECT 
            CAST(question_embedding AS JSON) as question_json,
            CAST(answer_embedding AS JSON) as answer_json 
        FROM data
    ")?;

    let rows = stmt.query_map([], |row| {
        let q_json: String = row.get(0)?;
        let a_json: String = row.get(1)?;
        
        // Chuyển đổi từ JSON string sang Vec<f32>
        let q_vec: Vec<f32> = serde_json::from_str(&q_json).unwrap();
        let a_vec: Vec<f32> = serde_json::from_str(&a_json).unwrap();
        
        Ok((q_vec, a_vec))
    })?;

    let embeddings = rows.filter_map(Result::ok).collect();
    Ok(embeddings)
}

fn main() -> Result<()> {
    let embeddings = query_db()?;
    
    println!("Tổng số cặp embedding: {}", embeddings.len());
    
    // In ra 2 cặp đầu tiên để kiểm tra
    for (i, (question, answer)) in embeddings.iter().take(2).enumerate() {
        println!("\n=== Cặp embedding thứ {} ===", i + 1);
        println!("\nQuestion embedding ({} chiều):", question.len());
        for (j, value) in question.iter().enumerate() {
            print!("{:.6} ", value);
            if (j + 1) % 10 == 0 { // Xuống dòng sau mỗi 10 giá trị
                println!();
            }
        }
        
        println!("\n\nAnswer embedding ({} chiều):", answer.len());
        for (j, value) in answer.iter().enumerate() {
            print!("{:.6} ", value);
            if (j + 1) % 10 == 0 { // Xuống dòng sau mỗi 10 giá trị
                println!();
            }
        }
        println!("\n"); // Thêm dòng trống giữa các cặp
    }
    
    Ok(())
}