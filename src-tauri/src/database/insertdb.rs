use duckdb::{Connection, Result};

pub fn insert_embeddings(question_embedding: Vec<f32>, answer_embedding: Vec<f32>) -> Result<()> {
    let conn = Connection::open("data.duckdb")?;
    
    let query = format!(
        "INSERT INTO data (question_embedding, answer_embedding) VALUES (array{:?}::REAL[], array{:?}::REAL[])",
        question_embedding, answer_embedding
    );
    
    conn.execute(&query, [])?;
    Ok(())
}

// fn simple_random() -> f32 {
//     let now = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .unwrap()
//         .subsec_nanos() as f32;
//     (now % 100.0) / 100.0 * 2.0 - 1.0
// }
// 
// fn generate_embedding(size: usize) -> Vec<f32> {
//     let mut vec = Vec::with_capacity(size);
//     for _ in 0..size {
//         vec.push(simple_random());
//     }
//     vec
// }

// fn main() {
//     // Sửa lại cách gọi hàm create_database
//     if let Err(e) = createdb::create_database() {
//         println!("Lỗi khi tạo database: {}", e);
//         return;
//     }
//     println!("Đã tạo database thành công!");

//     // Test với 3 cặp dữ liệu
//     let embedding_size = 5;
//     for i in 0..3 {
//         let question_embedding = generate_embedding(embedding_size);
//         let answer_embedding = generate_embedding(embedding_size);
        
//         match insert_embeddings(question_embedding.clone(), answer_embedding.clone()) {
//             Ok(_) => println!("Đã insert cặp dữ liệu thứ {} thành công!", i+1),
//             Err(e) => println!("Lỗi khi insert cặp dữ liệu thứ {}: {}", i+1, e)
//         }
        
//         println!("Question embedding: {:?}", question_embedding);
//         println!("Answer embedding: {:?}", answer_embedding);
//         println!("---");
//     }
// }

