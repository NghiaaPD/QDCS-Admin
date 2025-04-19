#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod functions;
mod database;
mod service;
mod middleware;

use crate::functions::process_docx::read_docx_content_from_bytes;
use crate::database::createdb::create_database;
use crate::database::insertdb::insert_embeddings;
use crate::service::querydb::query_db;
use crate::functions::cosine_similarity::calculate_cosine_similarity;
use crate::functions::plot_similarity::calculate_similarity_score;
use crate::middleware::check_duplicate_answers::{check_duplicate_answers, check_duplicates_within_question};
use crate::functions::load_accurancy::load_similarity_threshold;
use docx_rust::DocxFile;

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

#[tauri::command]
async fn process_docx(file_data: Vec<u8>) -> Result<String, String> {
    let _similarity_threshold = match load_similarity_threshold() {
        Ok(t) => t,
        Err(e) => {
            println!("Lỗi khi load threshold: {}", e);
            0.6
        }
    };
    
    match read_docx_content_from_bytes(&file_data) {
        Ok(questions) => {
            let db_result = query_db();
            
            match db_result {
                Ok(embeddings) => {
                    let mut results = Vec::new();
                    let mut processed_questions = std::collections::HashSet::new();
                    
                    let all_answers: Vec<String> = questions.iter()
                        .map(|q| q.correct_answer_text.clone())
                        .collect();
                    
                    let duplicate_answers = check_duplicate_answers(&all_answers);
                    
                    for (i, docx_item1) in questions.iter().enumerate() {
                        let mut found_similar = false;
                        
                        // Kiểm tra với các câu hỏi khác trong file
                        for (j, docx_item2) in questions.iter().enumerate() {
                            if i != j {
                                let question_similarity = calculate_cosine_similarity(
                                    &docx_item1.question_embedding,
                                    &docx_item2.question_embedding
                                );
                                
                                let answer_similarity = calculate_cosine_similarity(
                                    &docx_item1.answer_embedding,
                                    &docx_item2.answer_embedding
                                );
                                
                                if question_similarity > 0.5 && answer_similarity > 0.5 {
                                    results.push(serde_json::json!({
                                        "docx_question": docx_item1.text,
                                        "docx_answer": docx_item1.correct_answer_text,
                                        "answers": [],
                                        "correct_answer_keys": [],
                                        "true_answer": docx_item1.correct_answer_text,
                                        "similar_docx_question": docx_item2.text,
                                        "similar_docx_answer": docx_item2.correct_answer_text,
                                        "similarity_score": calculate_similarity_score(question_similarity, answer_similarity),
                                        "is_similar": true
                                    }));
                                    
                                    found_similar = true;
                                    processed_questions.insert(docx_item1.text.clone());
                                    processed_questions.insert(docx_item2.text.clone());
                                    break;
                                }
                            }
                        }
                        
                        if !found_similar && !processed_questions.contains(&docx_item1.text) {
                            let mut max_similarity = None;
                            
                            for db_item in &embeddings {
                                let question_similarity = calculate_cosine_similarity(
                                    &docx_item1.question_embedding,
                                    &db_item.0
                                );
                                
                                let answer_similarity = calculate_cosine_similarity(
                                    &docx_item1.answer_embedding,
                                    &db_item.1
                                );
                                
                                if question_similarity > 0.5 && answer_similarity > 0.5 {
                                    results.push(serde_json::json!({
                                        "docx_question": docx_item1.text,
                                        "docx_answer": docx_item1.correct_answer_text,
                                        "answers": [],
                                        "correct_answer_keys": [],
                                        "true_answer": docx_item1.correct_answer_text,
                                        "db_question": "Câu hỏi từ Database",
                                        "db_answer": "Đáp án từ Database",
                                        "similarity_score": calculate_similarity_score(question_similarity, answer_similarity),
                                        "is_similar": true
                                    }));
                                    
                                    processed_questions.insert(docx_item1.text.clone());
                                    found_similar = true;
                                    break;
                                } else {
                                    let current_avg = (question_similarity + answer_similarity) / 2.0;
                                    if let Some((_, _, prev_avg)) = &max_similarity {
                                        if current_avg > *prev_avg {
                                            max_similarity = Some((db_item, (question_similarity, answer_similarity), current_avg));
                                        }
                                    } else {
                                        max_similarity = Some((db_item, (question_similarity, answer_similarity), current_avg));
                                    }
                                }
                            }

                            if !found_similar {
                                if let Some((_db_item, (q_sim, a_sim), _)) = max_similarity {
                                    results.push(serde_json::json!({
                                        "docx_question": docx_item1.text,
                                        "docx_answer": docx_item1.correct_answer_text,
                                        "answers": [],
                                        "correct_answer_keys": [],
                                        "true_answer": docx_item1.correct_answer_text,
                                        "db_question": "Câu hỏi từ Database",
                                        "db_answer": "Đáp án từ Database",
                                        "similarity_score": calculate_similarity_score(q_sim, a_sim),
                                        "is_similar": false
                                    }));
                                    
                                    processed_questions.insert(docx_item1.text.clone());
                                }
                            }
                        }
                    }
                    
                    let result = if let Some((ans1, ans2, sim)) = duplicate_answers {
                        serde_json::json!({
                            "similarities": results,
                            "duplicate_answers": [ans1, ans2, sim]
                        })
                    } else {
                        serde_json::json!({
                            "similarities": results
                        })
                    };
                    
                    Ok(result.to_string())
                },
                Err(e) => Err(format!("Lỗi khi truy vấn database: {}", e))
            }
        },
        Err(e) => Err(format!("Lỗi khi đọc file: {}", e))
    }
}

#[tauri::command]
fn fill_format_check(file_data: Vec<u8>) -> Result<String, String> {
    use crate::functions::cosine_similarity::calculate_cosine_similarity;
    use crate::functions::plot_similarity::calculate_similarity_score;

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("temp_docx_check.docx");

    std::fs::write(&temp_file, &file_data)
        .map_err(|e| format!("Không thể tạo file tạm: {}", e))?;

    let file_path = temp_file.to_str()
        .ok_or("Không thể chuyển đổi đường dẫn file tạm")?;

    let _similarity_threshold = match load_similarity_threshold() {
        Ok(value) => value,
        Err(e) => {
            println!("Lỗi khi load threshold: {}", e);
            0.6 
        }
    };
    
    println!("Đang sử dụng threshold: {}", _similarity_threshold);

    let questions = crate::middleware::fill_format::read_docx_content(file_path)
        .map_err(|e| format!("Lỗi khi đọc file DOCX: {}", e))?;
    
    let _result_text = format!("Tổng số câu hỏi: {}\n\n", questions.len());
    let mut result_items = Vec::new();
    let mut duplicate_answers_info = Option::<(String, String, f32)>::None;

    let db_embeddings = match query_db() {
        Ok(embeddings) => embeddings,
        Err(e) => {
            println!("Lỗi khi truy vấn database: {}", e);
            Vec::new() 
        }
    };
    
    for (i, q1) in questions.iter().enumerate() {
        let mut is_similar = false;
        let mut similarity_score = 0.0;
        let mut similarity_type = "none"; 
        let mut similar_to = String::new();

        if let Some((ans1, ans2, sim)) = check_duplicates_within_question(q1) {
            duplicate_answers_info = Some((ans1.clone(), ans2.clone(), sim));
            is_similar = true;
            similarity_score = sim;
            similarity_type = "question";
            similar_to = format!("Trùng trong cùng câu hỏi: {} và {}", ans1, ans2);
        }
        else {
            for (j, q2) in questions.iter().enumerate() {
                if i != j {
                    let question_similarity = calculate_cosine_similarity(
                        &q1.question_embedding,
                        &q2.question_embedding
                    );
                    
                    let answer_similarity = calculate_cosine_similarity(
                        &q1.answer_embedding,
                        &q2.answer_embedding
                    );
                    
                    if question_similarity > _similarity_threshold && answer_similarity > _similarity_threshold {
                        is_similar = true;
                        similarity_score = calculate_similarity_score(question_similarity, answer_similarity);
                        similarity_type = "file"; 
                        similar_to = format!("Trùng trong file: {} và {}", q1.text, q2.text);
                        break;
                    }
                }
            }

            if !is_similar && !db_embeddings.is_empty() {
                let mut max_db_similarity = 0.0;
                
                for (db_q_embedding, db_a_embedding) in &db_embeddings {
                    let q_similarity = calculate_cosine_similarity(&q1.question_embedding, db_q_embedding);
                    let a_similarity = calculate_cosine_similarity(&q1.answer_embedding, db_a_embedding);
                    
                    let combined_similarity = calculate_similarity_score(q_similarity, a_similarity);
                    
                    if combined_similarity > max_db_similarity {
                        max_db_similarity = combined_similarity;
                        similar_to = format!("Trùng với câu hỏi trong database có độ tương đồng {:.2}%", combined_similarity * 100.0);
                    }
                }
                
                if max_db_similarity > _similarity_threshold {
                    is_similar = true;
                    similarity_score = max_db_similarity;
                    similarity_type = "database";
                    similar_to = format!("Trùng với câu hỏi trong database có độ tương đồng {:.2}%", max_db_similarity * 100.0);
                }
            }
        }

        let formatted_answers: Vec<String> = q1.answers.iter()
            .enumerate()
            .filter_map(|(i, ans)| {
                let trimmed = ans.trim();
                if trimmed.is_empty() {
                    return None;
                }

                let letter = char::from(b'a' + (i as u8 % 26));
                let _letter_prefix = format!("{}. ", letter);
                
                if trimmed == letter.to_string() || 
                   trimmed == format!("{}.", letter) ||
                   trimmed == format!("{}. {}", letter, letter) ||
                   trimmed == format!("{}. {}.", letter, letter) {
                    return None;
                }

                Some(format!("{}. {}", letter, trimmed))
            })
            .collect();

        let docx_answer: String;
        if q1.correct_answers.len() == 1 {
            docx_answer = q1.correct_answers[0].clone();
        } else {
            docx_answer = q1.correct_answers.join(", ");
        }

        let correct_answer_keys: Vec<String> = q1.correct_answer_keys
            .iter()
            .map(|key| key.to_lowercase())
            .collect();

        let item = serde_json::json!({
            "id": q1.id,
            "docx_question": q1.text,
            "docx_answer": docx_answer,
            "similarity_score": format!("{:.0}%", similarity_score * 100.0),
            "is_similar": is_similar,
            "answers": formatted_answers,
            "correct_answer_keys": correct_answer_keys,
            "correct_answers": q1.correct_answers,
            "similarity_type": similarity_type,
            "similar_to": similar_to
        });

        result_items.push(item);
    }
    
    let result = serde_json::json!({
        "similarities": result_items,
        "db_count": db_embeddings.len(),
        "duplicate_answers": duplicate_answers_info.map(|(a1, a2, sim)| vec![a1, a2, sim.to_string()]),
    });

    match serde_json::to_string(&result) {
        Ok(json_string) => Ok(json_string),
        Err(e) => Err(format!("Lỗi khi chuyển đổi kết quả sang JSON: {}", e)),
    }
}

#[tauri::command]
fn get_temp_file_path() -> String {
    // Lấy thư mục tạm của hệ thống
    let temp_dir = std::env::temp_dir();
    return temp_dir.to_string_lossy().to_string();
}

#[tauri::command]
async fn filter_docx(file_path: String, duplicate_ids: Vec<String>, original_filename: Option<String>) -> Result<String, String> {
    // Lấy thư mục hiện tại của ứng dụng
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Không thể xác định thư mục hiện tại: {}", e))?;

    println!("File path: {}", file_path);
    println!("Original filename: {:?}", original_filename);
    println!("IDs cần giữ lại: {:?}", duplicate_ids);

    // Kiểm tra xem có câu nào được giữ lại không
    if duplicate_ids.is_empty() {
        println!("Không có câu hỏi nào được giữ lại!");
        return Err("Không có câu hỏi nào được giữ lại sau khi lọc. Không thể tạo file mới.".to_string());
    }

    // Sử dụng tên file gốc nếu được cung cấp
    let output_file_stem = if let Some(original_name) = original_filename {
        let original_path = std::path::Path::new(&original_name);
        original_path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    } else {
        let path = std::path::Path::new(&file_path);
        path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    };

    // Lấy phần mở rộng
    let path = std::path::Path::new(&file_path);
    let extension = path.extension().unwrap_or_default().to_string_lossy();

    // Tạo đường dẫn file mới với timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let new_file_name = format!("{}_filtered_{}.{}", output_file_stem, timestamp, extension);
    let new_file_path = current_dir.join(&new_file_name);
    let new_file_path_str = new_file_path.to_str().unwrap().to_string();

    // Đảm bảo file nguồn tồn tại
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("File gốc không tồn tại: {}", file_path));
    }

    // Đọc và xử lý file DOCX
    use docx_rust::DocxFile;
    use docx_rust::document::{BodyContent};
    use std::fs::File;
    use std::io::Write;

    // Phương pháp 1: Đọc và viết lại với thư viện docx-rust
    let _success = false;
    
    // Xử lý các định dạng màu (loại bỏ oklch)
    let mut xml_content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Không thể đọc file DOCX như là văn bản: {}", e))?;
    
    // Thay thế các định dạng màu oklch bằng màu đen mặc định
    xml_content = xml_content.replace("oklch(", "rgb(0,0,0");
    
    // Ghi nội dung đã sửa vào file tạm
    let temp_file_path = format!("{}_temp_{}.{}", output_file_stem, timestamp, extension);
    let mut temp_file = File::create(&temp_file_path)
        .map_err(|e| format!("Không thể tạo file tạm: {}", e))?;
    temp_file.write_all(xml_content.as_bytes())
        .map_err(|e| format!("Không thể ghi vào file tạm: {}", e))?;
    
    // Sử dụng file đã được sửa màu sắc để tiếp tục xử lý
    let doc_file = DocxFile::from_file(&temp_file_path)
        .map_err(|e| format!("Lỗi khi mở file DOCX: {}", e))?;
    
    let mut docx = doc_file.parse()
        .map_err(|e| format!("Lỗi khi phân tích DOCX: {}", e))?;
    
    // Lọc các bảng (table) trong DOCX dựa trên IDs
    let mut filtered_body_content = Vec::new();
    
    for content in &docx.document.body.content {
        if let BodyContent::Table(table) = content {
            // Kiểm tra xem table có phải là câu hỏi không
            if let Some(first_row) = table.rows.first() {
                if first_row.cells.len() >= 2 {
                    // Thử lấy ID từ ô đầu tiên
                    if let Some(cell) = first_row.cells.first() {
                        use docx_rust::document::{TableRowContent, TableCellContent, ParagraphContent, RunContent};
                        
                        // Extract ID
                        let mut id = String::new();
                        if let TableRowContent::TableCell(cell_data) = cell {
                            for content in &cell_data.content {
                                if let TableCellContent::Paragraph(p) = content {
                                    for run in &p.content {
                                        if let ParagraphContent::Run(r) = run {
                                            for text in &r.content {
                                                if let RunContent::Text(t) = text {
                                                    id = t.text.trim().to_string();
                                                    // Nếu ID có định dạng "QN=123", lấy phần sau "QN="
                                                    if let Some(id_part) = id.strip_prefix("QN=") {
                                                        id = id_part.trim().to_string();
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Nếu ID nằm trong danh sách các ID cần giữ lại, thì giữ lại table này
                        if duplicate_ids.contains(&id) {
                            filtered_body_content.push(BodyContent::Table(table.clone()));
                        }
                    }
                }
            }
        } else {
            // Giữ lại các nội dung không phải là table
            filtered_body_content.push(content.clone());
        }
    }
    
    // Cập nhật nội dung body trong DOCX
    docx.document.body.content = filtered_body_content;
    
    // Ghi ra file mới
    match docx.write_file(&new_file_path_str) {
        Ok(_) => {
            let _ = ();
            println!("Đã lọc và lưu file DOCX thành công (phương pháp 1)");
        },
        Err(e) => {
            println!("Lỗi khi ghi file DOCX (phương pháp 1): {}", e);
        }
    }
    
    // Xóa file tạm
    let _ = std::fs::remove_file(&temp_file_path);
    
    Ok(new_file_path_str)
}

#[tauri::command]
async fn filter_docx_with_data(file_data: Vec<u8>, duplicate_ids: Vec<String>, original_filename: Option<String>) -> Result<String, String> {
    use docx_rust::document::{BodyContent};

    // Lấy thư mục hiện tại của ứng dụng
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Không thể xác định thư mục hiện tại: {}", e))?;

    // Kiểm tra xem có câu nào được giữ lại không
    if duplicate_ids.is_empty() {
        println!("Không có câu hỏi nào được giữ lại!");
        return Err("Không có câu hỏi nào được giữ lại sau khi lọc. Không thể tạo file mới.".to_string());
    }

    // Sử dụng tên file gốc nếu được cung cấp
    let output_file_stem = if let Some(ref original_name) = original_filename {
        let original_path = std::path::Path::new(original_name);
        original_path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    } else {
        "filtered_docx".to_string()
    };

    // Tạo đường dẫn file mới với timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let new_file_name = format!("{}_filtered_{}.docx", output_file_stem, timestamp);
    let new_file_path = current_dir.join(&new_file_name);
    let new_file_path_str = new_file_path.to_str().unwrap().to_string();

    // Tạo file tạm để lưu dữ liệu
    let temp_file_path = std::env::temp_dir().join(format!("temp_file_{}.docx", timestamp));
    let temp_file_path_str = temp_file_path.to_str().unwrap().to_string();
    
    // Ghi dữ liệu vào file tạm
    match std::fs::write(&temp_file_path, &file_data) {
        Ok(_) => println!("Đã ghi file tạm thành công: {}", temp_file_path_str),
        Err(e) => return Err(format!("Không thể ghi file tạm: {}", e))
    }

    // PHƯƠNG PHÁP ĐƠN GIẢN: Triển khai trực tiếp docx-rust tại đây
    let _success = false;
    
    // Thử phương pháp với docx-rust
    {
        use docx_rust::document::{TableRowContent, TableCellContent, ParagraphContent, RunContent};
        
        let doc_file = match DocxFile::from_file(&temp_file_path_str) {
            Ok(file) => file,
            Err(e) => {
                // Thử sao chép file về đích khi có lỗi
                if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path) {
                    let _ = true;
                }
                return Err(format!("Không thể đọc file DOCX: {}", e))
            }
        };
        
        let mut docx = match doc_file.parse() {
            Ok(parsed) => parsed,
            Err(e) => {
                // Thử sao chép file về đích khi có lỗi
                if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path) {
                    let _ = true;
                }
                return Err(format!("Lỗi khi phân tích DOCX: {}", e))
            }
        };
        
        // Lọc các bảng (table) trong DOCX dựa trên IDs
        let mut filtered_body_content = Vec::new();
        
        for content in &docx.document.body.content {
            if let BodyContent::Table(table) = content {
                let mut keep_table = false;
                
                // Duyệt qua các hàng và ô để tìm ID
                for row in &table.rows {
                    for cell_content in &row.cells {
                        if let TableRowContent::TableCell(cell) = cell_content {
                            for cell_item in &cell.content {
                                if let TableCellContent::Paragraph(para) = cell_item {
                                    // Lấy tất cả text từ paragraph
                                    let mut id_text = String::new();
                                    for para_content in &para.content {
                                        if let ParagraphContent::Run(run) = para_content {
                                            for run_content in &run.content {
                                                if let RunContent::Text(text_content) = run_content {
                                                    id_text.push_str(&text_content.text);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Trim ID và xử lý QN= prefix
                                    let trimmed_id = id_text.trim();
                                    let id = if let Some(id_part) = trimmed_id.strip_prefix("QN=") {
                                        id_part.trim()
                                    } else {
                                        trimmed_id
                                    };
                                    
                                    if duplicate_ids.contains(&id.to_string()) {
                                        keep_table = true;
                                        break;
                                    }
                                }
                            }
                            if keep_table {
                                break;
                            }
                        }
                    }
                    if keep_table {
                        break;
                    }
                }
                
                if keep_table {
                    filtered_body_content.push(BodyContent::Table(table.clone()));
                }
            } else {
                // Giữ lại các nội dung không phải là table
                filtered_body_content.push(content.clone());
            }
        }
        
        // Cập nhật nội dung body trong DOCX
        docx.document.body.content = filtered_body_content;
        
        // Ghi ra file mới
        match docx.write_file(&new_file_path_str) {
            Ok(_) => {
                let _ = ();
            },
            Err(e) => {
                // Thử sao chép file về đích khi có lỗi
                if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path) {
                    let _ = ();
                }
            }
        }
    }
    
    // Xóa file tạm
    let _ = std::fs::remove_file(&temp_file_path);
    
    Ok(new_file_path_str)
}

#[cfg(not(feature = "test_fill_format"))]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            read_docx, 
            process_docx, 
            fill_format_check, 
            filter_docx,
            filter_docx_with_data,
            get_temp_file_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(feature = "test_fill_format")]
fn main() {
    use std::env;
    use std::io;
    use std::process;
    use serde_json::Value;
    
    println!("=== CHẾ ĐỘ KIỂM THỬ FILL_FORMAT_CHECK ===");
    
    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("Nhập đường dẫn đến file DOCX để kiểm tra:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Không thể đọc input");
        input.trim().to_string()
    };

    if !fs::metadata(&file_path).is_ok() {
        eprintln!("Lỗi: File không tồn tại: {}", file_path);
        process::exit(1);
    }
    
    println!("Đang kiểm tra file: {}", file_path);

    let file_data = match fs::read(&file_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Lỗi khi đọc file: {}", e);
            process::exit(1);
        }
    };
    
    // Gọi hàm fill_format_check
    match fill_format_check(file_data) {
        Ok(json_str) => {
            println!("\n=== KẾT QUẢ KIỂM TRA ===");

            let result: Value = serde_json::from_str(&json_str).unwrap_or_else(|e| {
                eprintln!("Lỗi phân tích JSON: {}", e);
                process::exit(1);
            });

            if let Some(similarities) = result["similarities"].as_array() {
                for (i, item) in similarities.iter().enumerate() {
                    println!("\n--- Câu hỏi {} ---", i + 1);
                    println!("ID: '{}'", item["id"].as_str().unwrap_or(""));
                    println!("Nội dung: '{}'", item["docx_question"].as_str().unwrap_or(""));
                    
                    println!("Đáp án:");
                    if let Some(answers) = item["answers"].as_array() {
                        for (j, ans) in answers.iter().enumerate() {
                            println!("  {}. {}", j + 1, ans.as_str().unwrap_or(""));
                        }
                    }
                    
                    println!("Đáp án đúng: {}", item["docx_answer"].as_str().unwrap_or(""));

                    println!("TAG TRÙNG: {}", item["similarity_type"].as_str().unwrap_or("không trùng lặp"));
                }
            }
            
            println!("\nKiểm tra hoàn tất!");
        },
        Err(e) => {
            eprintln!("Lỗi trong quá trình kiểm tra: {}", e);
            process::exit(1);
        }
    }
}