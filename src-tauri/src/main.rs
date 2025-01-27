#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod functions;
mod database;
use crate::functions::process_docx::read_docx_content_from_bytes;
use crate::database::createdb::create_database;
use crate::database::insertdb::insert_embeddings;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Xin chào, {}!", name)
}

#[tauri::command]
async fn read_docx(file_data: Vec<u8>) -> Result<String, String> {
    #[allow(non_snake_case)]
    let fileData = file_data;
    
    if let Err(e) = create_database() {
        return Err(format!("Lỗi khi tạo database: {}", e));
    }
    
    match read_docx_content_from_bytes(&fileData) {
        Ok(questions) => {
            let mut result_messages = Vec::new();
            
            for (i, q) in questions.iter().enumerate() {
                match insert_embeddings(q.question_embedding.clone(), q.answer_embedding.clone()) {
                    Ok(_) => {
                        result_messages.push(format!(
                            "Câu hỏi {} đã lưu vào database thành công!",
                            i + 1
                        ));
                    },
                    Err(e) => {
                        return Err(format!("Lỗi khi lưu vào database câu hỏi {}: {}", i + 1, e));
                    }
                }
            }
            
            Ok(result_messages.join("\n\n"))
        },
        Err(e) => Err(format!("Lỗi khi đọc file: {}", e))
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, read_docx])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}