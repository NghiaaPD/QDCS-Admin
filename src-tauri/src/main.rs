#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod functions;
use crate::functions::process_docx::read_docx_content_from_bytes;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Xin chào, {}!", name)
}

#[tauri::command]
async fn read_docx(file_data: Vec<u8>) -> Result<String, String> {
    #[allow(non_snake_case)]
    let fileData = file_data; // Tạo alias cho tham số
    match read_docx_content_from_bytes(&fileData) {
        Ok(questions) => {
            let content = questions.iter()
                .enumerate()
                .map(|(i, q)| {
                    format!("Câu hỏi {}: {}\nĐáp án đúng: {}\n",
                        i + 1,
                        q.text,
                        q.correct_answer_text
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");
            Ok(content)
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