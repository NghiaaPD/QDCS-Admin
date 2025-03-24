use docx_rust::DocxFile;
use docx_rust::document::{BodyContent, ParagraphContent, RunContent, TableRowContent, TableCellContent};
use std::path::Path;
use std::sync::LazyLock;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::path::PathBuf;

pub static EMBEDDING_MODEL: LazyLock<TextEmbedding> = LazyLock::new(|| {
    let mut options = InitOptions::default();
    options.model_name = EmbeddingModel::AllMiniLML6V2;
    options.show_download_progress = true;
    options.cache_dir = PathBuf::from("FUC-mini");
    
    TextEmbedding::try_new(options)
        .expect("Failed to init embedding model")
});

pub fn extract_cell_text(cell: &TableRowContent) -> String {
    match cell {
        TableRowContent::TableCell(cell_data) => {
            cell_data.content.iter().fold(String::new(), |acc, content| {
                match content {
                    TableCellContent::Paragraph(p) => {
                        acc + &p.content.iter().fold(String::new(), |acc, run| {
                            if let ParagraphContent::Run(r) = run {
                                acc + &r.content.iter().fold(String::new(), |acc, text| {
                                    if let RunContent::Text(t) = text {
                                        acc + &t.text
                                    } else {
                                        acc
                                    }
                                })
                            } else {
                                acc
                            }
                        })
                    }
                }
            })
        },
        _ => String::new()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub answers: Vec<String>,
    pub correct_answers: Vec<String>,
    pub correct_answer_keys: Vec<String>,
    pub question_embedding: Vec<f32>,
    pub answer_embedding: Vec<f32>,
}

pub fn read_docx_content(file_path: &str) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    if !Path::new(file_path).exists() {
        return Err("File không tồn tại!".into());
    }

    let doc_file = DocxFile::from_file(file_path)?;
    let docx = doc_file.parse()?;
    let mut questions = Vec::new();

    for element in &docx.document.body.content {
        if let BodyContent::Table(table) = element {
            let mut question = Question {
                id: String::new(),
                text: String::new(),
                answers: Vec::new(),
                correct_answers: Vec::new(),
                correct_answer_keys: Vec::new(),
                question_embedding: Vec::new(),
                answer_embedding: Vec::new(),
            };

            // Thu thập câu hỏi từ hàng đầu tiên
            if let Some(first_row) = table.rows.first() {
                for (cell_idx, cell) in first_row.cells.iter().enumerate() {
                    let cell_text = extract_cell_text(cell).trim().to_string();
                    match cell_idx {
                        0 => if let Some(id) = cell_text.strip_prefix("QN=") {
                            question.id = id.trim().to_string();
                        },
                        1 => question.text = cell_text,
                        _ => {}
                    }
                }
            }

            // Map để lưu trữ các câu trả lời theo key (A, B, C, D)
            let mut answer_texts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            let mut correct_answer_keys = Vec::new();

            // Thu thập các câu trả lời và ANSWER
            for row in table.rows.iter() {
                let first_cell = extract_cell_text(&row.cells[0]).trim().to_string();
                
                // Nếu là dòng câu trả lời (a., b., c., d.)
                if first_cell.len() == 2 && first_cell.ends_with('.') {
                    let option_key = first_cell.chars().next().unwrap().to_uppercase().to_string();
                    if let Some(cell) = row.cells.get(1) {
                        let answer_text = extract_cell_text(cell).trim().to_string();
                        answer_texts.insert(option_key.clone(), answer_text.clone());
                        question.answers.push(format!("{} {}", first_cell, answer_text));
                    }
                }
                
                // Nếu là dòng ANSWER
                if first_cell == "ANSWER:" {
                    if let Some(cell) = row.cells.get(1) {
                        let answer_text = extract_cell_text(cell).trim().to_uppercase();
                        correct_answer_keys = answer_text.split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                        question.correct_answer_keys = correct_answer_keys.clone();
                    }
                }
            }

            // Lấy tất cả câu trả lời đúng từ map
            for key in correct_answer_keys {
                if let Some(answer_text) = answer_texts.get(&key) {
                    question.correct_answers.push(answer_text.clone());
                }
            }

            // Kiểm tra và tạo embeddings
            if question.id.is_empty() {
                return Err("File sai format: Thiếu ID câu hỏi (QN=)".into());
            }
            if question.text.is_empty() {
                return Err(format!("File sai format: Câu hỏi {} thiếu nội dung", question.id).into());
            }
            if question.correct_answers.is_empty() {
                return Err(format!("File sai format: Câu hỏi {} thiếu đáp án đúng", question.id).into());
            }

            // Tạo embedding không log
            question.question_embedding = EMBEDDING_MODEL.embed(
                vec![&question.text], 
                None
            )?.remove(0);
            
            // Tạo embedding không log
            let combined_answers = question.correct_answers.join(" ");
            question.answer_embedding = EMBEDDING_MODEL.embed(
                vec![&combined_answers], 
                None
            )?.remove(0);

            questions.push(question);
        }
    }
    
    Ok(questions)
}

// Thêm hàm main để chạy trực tiếp fill_format.rs
// #[cfg(not(any(test, feature = "library")))]
// fn main() {
//     use std::env;
//     use std::process;
    
//     // Lấy đường dẫn file từ tham số dòng lệnh
//     let args: Vec<String> = env::args().collect();
    
//     // Hiển thị hướng dẫn nếu không cung cấp đường dẫn
//     if args.len() < 2 {
//         println!("Cách sử dụng: cargo run --bin fill_format -- <đường_dẫn_file.docx>");
//         process::exit(1);
//     }
    
//     let file_path = &args[1];
//     println!("Đang phân tích file: {}", file_path);
    
//     // Phân tích file DOCX
//     match read_docx_content(file_path) {
//         Ok(questions) => {
//             println!("\n=== KẾT QUẢ PHÂN TÍCH ===");
//             println!("Tổng số câu hỏi: {}", questions.len());
            
//             // In chi tiết từng câu hỏi
//             for (i, q) in questions.iter().enumerate() {
//                 println!("\n--- Câu hỏi {} ---", i + 1);
//                 println!("ID: '{}'", q.id);
//                 println!("Nội dung: '{}'", q.text);
//                 println!("Đáp án:");
//                 for (j, ans) in q.answers.iter().enumerate() {
//                     println!("  {}. {}", j + 1, ans);
//                 }
//                 println!("Đáp án đúng (keys): {:?}", q.correct_answer_keys);
//                 println!("Đáp án đúng (text): {:?}", q.correct_answers);
                
//                 // In chiều dài embedding để kiểm tra
//                 println!("Question embedding length: {}", q.question_embedding.len());
//                 println!("Answer embedding length: {}", q.answer_embedding.len());
//             }
            
//             // In thông báo thành công
//             println!("\nPhân tích thành công {} câu hỏi!", questions.len());
//         },
//         Err(e) => {
//             eprintln!("Lỗi khi phân tích file: {}", e);
//             process::exit(1);
//         }
//     }
// }