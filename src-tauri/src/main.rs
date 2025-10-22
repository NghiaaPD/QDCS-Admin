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
use docx_rust::document::{BodyContent, Paragraph, ParagraphContent, Run, RunContent, Text };
use crate::middleware::fill_format::extract_cell_text;

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
                let _letter_prefix = format!("{}.", letter);
                
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
async fn filter_docx(file_path: String, duplicate_ids: Vec<String>, _original_filename: Option<String>) -> Result<String, String> {
    let doc_file = DocxFile::from_file(&file_path)
        .map_err(|e| format!("Lỗi khi mở file DOCX: {}", e))?;
    let docx = doc_file.parse()
        .map_err(|e| format!("Lỗi khi phân tích DOCX: {}", e))?;

    let mut kept_ids = Vec::new();

    for content in &docx.document.body.content {
        if let BodyContent::Table(table) = content {
            if let Some(first_row) = table.rows.first() {
                if let Some(cell) = first_row.cells.first() {
                    let cell_text = extract_cell_text(cell).trim().to_string();
                    if let Some(id) = cell_text.strip_prefix("QN=") {
                        let id = id.trim().to_string();
                        if duplicate_ids.contains(&id) {
                            kept_ids.push(id.clone());
                            println!("Giữ lại bảng với QN={}", id);
                        } else {
                            println!("Loại bỏ bảng với QN={}", id);
                        }
                    } else {
                        println!("Bảng không có QN=, bỏ qua");
                    }
                }
            }
        }
    }

    println!("Các QN= id được giữ lại: {:?}", kept_ids);

    Ok(format!("Các QN= id được giữ lại: {:?}", kept_ids))
}

#[tauri::command]
async fn filter_docx_with_data(file_data: Vec<u8>, duplicate_ids: Vec<String>, original_filename: Option<String>) -> Result<String, String> {
    use docx_rust::document::{BodyContent};

    // println!("Danh sách duplicate_ids FE gửi lên: {:?}", duplicate_ids);

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
    let current_dir = std::env::current_dir().unwrap();
    let new_file_path = current_dir.join(&new_file_name);
    let new_file_path_str = new_file_path.to_str().unwrap().to_string();

    // Tạo file tạm để lưu dữ liệu
    let temp_file_path = std::env::temp_dir().join(format!("temp_file_{}.docx", timestamp));
    let temp_file_path_str = temp_file_path.to_str().unwrap().to_string();
    
    // Ghi dữ liệu vào file tạm
    match std::fs::write(&temp_file_path, &file_data) {
        Ok(_) => {}, // println!("Đã ghi file tạm thành công: {}", temp_file_path_str),
        Err(e) => return Err(format!("Không thể ghi file tạm: {}", e))
    }

    // Đọc và parse file DOCX
    let doc_file = match DocxFile::from_file(&temp_file_path_str) {
        Ok(file) => file,
        Err(e) => {
            // Thử sao chép file về đích khi có lỗi
            if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path_str) {
                let _ = true;
            }
            return Err(format!("Không thể đọc file DOCX: {}", e))
        }
    };
    
    let mut docx = match doc_file.parse() {
        Ok(parsed) => parsed,
        Err(e) => {
            // Thử sao chép file về đích khi có lỗi
            if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path_str) {
                let _ = true;
            }
            return Err(format!("Lỗi khi phân tích DOCX: {}", e))
        }
    };

    let mut filtered_body_content: Vec<BodyContent> = Vec::new();
    let mut kept_count = 0;
    let mut removed_count = 0;
    let mut kept_ids = Vec::new();
    let mut removed_ids = Vec::new();

    // Log thông tin debug
    // println!("Tổng số phần tử trong body: {}", docx.document.body.content.len());

    // Lọc các bảng (table) trong DOCX dựa trên IDs
    for content in &docx.document.body.content {
        if let BodyContent::Table(table) = content {
            let mut found_id = None;
            
            // Chỉ kiểm tra cell đầu tiên của hàng đầu tiên (như fill_format.rs)
            if let Some(first_row) = table.rows.first() {
                if let Some(cell) = first_row.cells.first() {
                    let cell_text = extract_cell_text(cell).trim().to_string();
                    // println!("Cell đầu tiên của bảng: '{}'", cell_text);
                    if let Some(id) = cell_text.strip_prefix("QN=") {
                        found_id = Some(id.trim().to_string());
                        // println!("==> ĐÃ TÌM THẤY QN= ID: '{}'", id.trim());
                    }
                }
            }
            
            if let Some(id) = found_id {
                if duplicate_ids.contains(&id) {
                    filtered_body_content.push(BodyContent::Table(table.clone()));
                    kept_count += 1;
                    kept_ids.push(id.clone());
                    // println!("Giữ lại bảng với QN={}", id);
                } else {
                    removed_count += 1;
                    removed_ids.push(id.clone());
                    // println!("Đã loại bỏ table với QN={}", id);
                }
            } else {
                removed_count += 1;
                // println!("Đã loại bỏ table không có QN=");
            }
        } else {
            // Giữ lại các nội dung không phải là table
            filtered_body_content.push(content.clone());
        }
    }
    
    println!("Tổng kết: Giữ lại {} bảng, loại bỏ {} bảng", kept_count, removed_count);

    // Tạo các paragraph thông báo để thêm vào đầu document
    let mut notification_content: Vec<BodyContent> = Vec::new();

    // Tạo thông báo số lượng câu trùng
    let kept_text = if kept_count > 0 {
        format!("Số lượng câu không trùng (được giữ lại): {} câu - ID: {}", 
                kept_count, 
                kept_ids.join(", "))
    } else {
        "Số lượng câu không trùng (được giữ lại): 0 câu".to_string()
    };

    let removed_text = if removed_count > 0 {
        format!("Số lượng câu trùng (đã loại bỏ): {} câu - ID: {}", 
                removed_count, 
                removed_ids.join(", "))
    } else {
        "Số lượng câu trùng (đã loại bỏ): 0 câu".to_string()
    };

    // Tạo paragraph cho thông báo câu không trùng
    let kept_paragraph = Paragraph {
        content: vec![
            ParagraphContent::Run(Run {
                content: vec![
                    RunContent::Text(Text {
                        text: kept_text.into(),
                        space: None,
                    })
                ],
                ..Default::default()
            })
        ],
        ..Default::default()
    };

    // Tạo paragraph cho thông báo câu trùng
    let removed_paragraph = Paragraph {
        content: vec![
            ParagraphContent::Run(Run {
                content: vec![
                    RunContent::Text(Text {
                        text: removed_text.into(),
                        space: None,
                    })
                ],
                ..Default::default()
            })
        ],
        ..Default::default()
    };

    // Tạo paragraph trống để ngăn cách
    let empty_paragraph = Paragraph {
        content: vec![
            ParagraphContent::Run(Run {
                content: vec![
                    RunContent::Text(Text {
                        text: "".into(),
                        space: None,
                    })
                ],
                ..Default::default()
            })
        ],
        ..Default::default()
    };

    // Thêm các paragraph thông báo vào đầu document
    notification_content.push(BodyContent::Paragraph(kept_paragraph));
    notification_content.push(BodyContent::Paragraph(removed_paragraph));
    notification_content.push(BodyContent::Paragraph(empty_paragraph));

    // Kết hợp notification với filtered content
    notification_content.extend(filtered_body_content);

    // Cập nhật nội dung body trong DOCX
    docx.document.body.content = notification_content;
    
    // Ghi ra file mới
    match docx.write_file(&new_file_path_str) {
        Ok(_) => {
            println!("Đã xuất file thành công: {}", new_file_path_str);
        },
        Err(e) => {
            println!("Lỗi khi ghi file DOCX: {}", e);
            // Thử sao chép file gốc về đích khi có lỗi
            if let Ok(_) = std::fs::copy(&temp_file_path, &new_file_path_str) {
                println!("Đã sao chép file gốc thay thế");
            }
            return Err(format!("Lỗi khi ghi file DOCX: {}", e));
        }
    }
    
    // Xóa file tạm
    let _ = std::fs::remove_file(&temp_file_path);
    
    Ok(new_file_path_str)
}

#[tauri::command]
fn backup_duckdb() -> Result<String, String> {
    let db_path = "data.duckdb";
    let backup_path = "new_data.duckdb";

    // Thực hiện checkpoint để flush dữ liệu
    if let Ok(conn) = duckdb::Connection::open(db_path) {
        let _ = conn.execute("PRAGMA checkpoint;", []);
    }

    std::fs::copy(db_path, backup_path)
        .map_err(|e| format!("Không thể sao lưu database: {}", e))?;
    Ok(backup_path.to_string())
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
            get_temp_file_path,
            backup_duckdb,
            insert_filtered_to_new_db  // <-- Thêm dòng này
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

#[tauri::command]
async fn insert_filtered_to_new_db() -> Result<String, String> {
    // 1. Tìm file filtered mới nhất
    let filtered_file_path = match find_latest_filtered_file() {
        Ok(path) => path,
        Err(e) => return Err(format!("Không tìm thấy file filtered: {}", e))
    };
    
    println!("Tìm thấy file filtered mới nhất: {}", filtered_file_path);
    
    // 2. Tạo bản sao database
    match create_new_database_copy() {
        Ok(_) => println!("Đã tạo bản sao new_data.duckdb thành công"),
        Err(e) => return Err(format!("Không thể tạo bản sao database: {}", e))
    };
    
    // 3. Đọc và insert dữ liệu từ file filtered vào new_data.duckdb
    match insert_filtered_data_to_new_db(&filtered_file_path).await {
        Ok(message) => {
            println!("{}", message);
            Ok(format!("Đã insert dữ liệu từ {} vào new_data.duckdb thành công", filtered_file_path))
        },
        Err(e) => Err(format!("Lỗi khi insert dữ liệu: {}", e))
    }
}

// Helper function: Tìm file filtered mới nhất
fn find_latest_filtered_file() -> Result<String, String> {
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Không thể lấy thư mục hiện tại: {}", e))?;
    
    let mut filtered_files = Vec::new();
    
    // Đọc tất cả file trong thư mục hiện tại
    let entries = std::fs::read_dir(&current_dir)
        .map_err(|e| format!("Không thể đọc thư mục: {}", e))?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // Tìm file có pattern *_filtered_*.docx
            if file_name.contains("_filtered_") && file_name.ends_with(".docx") {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        filtered_files.push((file_name, modified));
                    }
                }
            }
        }
    }
    
    if filtered_files.is_empty() {
        return Err("Không tìm thấy file filtered nào".to_string());
    }
    
    // Sắp xếp theo thời gian modified (mới nhất trước)
    filtered_files.sort_by(|a, b| b.1.cmp(&a.1));
    
    let latest_file = &filtered_files[0].0;
    let full_path = current_dir.join(latest_file);
    
    Ok(full_path.to_string_lossy().to_string())
}

// Helper function: Tạo bản sao database
fn create_new_database_copy() -> Result<(), String> {
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Không thể lấy thư mục hiện tại: {}", e))?;
    
    let source_path = current_dir.join("data.duckdb");
    let backup_path = current_dir.join("new_data.duckdb");
    
    // Kiểm tra xem file data.duckdb có tồn tại không
    if !source_path.exists() {
        return Err("File data.duckdb không tồn tại".to_string());
    }
    
    // Copy file data.duckdb thành new_data.duckdb
    std::fs::copy(&source_path, &backup_path)
        .map_err(|e| format!("Không thể copy database: {}", e))?;
    
    Ok(())
}

// Helper function: Insert dữ liệu từ file filtered vào new_data.duckdb
async fn insert_filtered_data_to_new_db(file_path: &str) -> Result<String, String> {
    // Đọc file DOCX filtered
    let file_data = std::fs::read(file_path)
        .map_err(|e| format!("Không thể đọc file filtered: {}", e))?;
    
    // Parse DOCX và lấy dữ liệu câu hỏi
    match read_docx_content_from_bytes(&file_data) {
        Ok(questions) => {
            let mut success_count = 0;
            let mut error_count = 0;
            
            for (i, q) in questions.iter().enumerate() {
                match insert_embeddings_to_new_database(q.question_embedding.clone(), q.answer_embedding.clone()) {
                    Ok(_) => {
                        success_count += 1;
                        println!("Đã insert câu hỏi {} thành công", i + 1);
                    },
                    Err(e) => {
                        error_count += 1;
                        println!("Lỗi khi insert câu hỏi {}: {}", i + 1, e);
                    }
                }
            }
            
            Ok(format!(
                "Đã insert {} câu hỏi thành công, {} lỗi vào new_data.duckdb", 
                success_count, 
                error_count
            ))
        },
        Err(e) => Err(format!("Lỗi khi đọc nội dung file filtered: {}", e))
    }
}

// Helper function: Insert embeddings vào new_data.duckdb (theo format insertdb.rs)
fn insert_embeddings_to_new_database(question_embedding: Vec<f32>, answer_embedding: Vec<f32>) -> Result<(), String> {
    use duckdb::Connection;
    
    let conn = Connection::open("new_data.duckdb")
        .map_err(|e| format!("Không thể mở new_data.duckdb: {}", e))?;
    
    // Sử dụng format tương tự insertdb.rs
    let query = format!(
        "INSERT INTO data (question_embedding, answer_embedding) VALUES (array{:?}::REAL[], array{:?}::REAL[])",
        question_embedding, answer_embedding
    );
    
    conn.execute(&query, [])
        .map_err(|e| format!("Không thể insert vào new_data.duckdb: {}", e))?;
    
    Ok(())
}