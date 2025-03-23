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
use crate::service::load_accurancy::load_similarity_threshold;

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
    let similarity_threshold = match load_similarity_threshold() {
        Ok(t) => t,
        Err(e) => return Err(format!("Lỗi khi đọc threshold: {}", e)),
    };
    
    match read_docx_content_from_bytes(&file_data) {
        Ok(questions) => {
            let db_result = query_db();
            
            match db_result {
                Ok(embeddings) => {
                    let mut results = Vec::new();
                    let mut processed_questions = std::collections::HashSet::new();
                    
                    // Kiểm tra trùng lặp câu trả lời trong file
                    let all_answers: Vec<String> = questions.iter()
                        .map(|q| q.correct_answer_text.clone())
                        .collect();
                    
                    // Kiểm tra trùng câu trả lời
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
                        
                        // Nếu chưa tìm thấy trùng lặp trong file, kiểm tra với database
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
                            
                            // Nếu không tìm thấy trùng lặp nào
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
    
    // Tạo file tạm thời
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("temp_docx_check.docx");
    
    // Ghi dữ liệu vào file tạm
    std::fs::write(&temp_file, &file_data)
        .map_err(|e| format!("Không thể tạo file tạm: {}", e))?;
    
    // Chuyển đổi path thành chuỗi
    let file_path = temp_file.to_str()
        .ok_or("Không thể chuyển đổi đường dẫn file tạm")?;
    
    // Lấy threshold từ cấu hình
    let threshold = load_similarity_threshold()
        .unwrap_or(0.6); // Giảm xuống 0.6 để phát hiện trùng lặp tốt hơn
    
    // Đọc nội dung file DOCX
    let questions = crate::middleware::fill_format::read_docx_content(file_path)
        .map_err(|e| format!("Lỗi khi đọc file DOCX: {}", e))?;
    
    let mut result_text = format!("Tổng số câu hỏi: {}\n\n", questions.len());
    let mut result_items = Vec::new();
    let mut duplicate_answers_info = Option::<(String, String, f32)>::None;

    // Truy vấn dữ liệu từ database (cặp question_embedding và answer_embedding)
    let db_embeddings = match query_db() {
        Ok(embeddings) => embeddings,
        Err(e) => {
            println!("Lỗi khi truy vấn database: {}", e);
            Vec::new() // Nếu không truy vấn được DB, sử dụng vector rỗng
        }
    };
    
    // Phân tích từng câu hỏi và kiểm tra trùng lặp
    for (i, q1) in questions.iter().enumerate() {
        let mut is_similar = false;
        let mut similarity_score = 0.0;
        let mut similarity_type = "none"; // Mặc định là không trùng
        let mut similar_to = String::new();

        // 1. Kiểm tra trùng đáp án trong cùng câu hỏi
        if let Some((ans1, ans2, sim)) = check_duplicates_within_question(q1) {
            duplicate_answers_info = Some((ans1.clone(), ans2.clone(), sim));
            is_similar = true;
            similarity_score = sim;
            similarity_type = "question"; // Trùng trong cùng câu
            similar_to = format!("Trùng trong cùng câu hỏi: {} và {}", ans1, ans2);
        }
        // 2. Nếu không trùng trong câu, kiểm tra với câu khác trong file
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
                    
                    if question_similarity > threshold && answer_similarity > threshold {
                        is_similar = true;
                        similarity_score = calculate_similarity_score(question_similarity, answer_similarity);
                        similarity_type = "file"; // Trùng trong file
                        similar_to = format!("Trùng trong file: {} và {}", q1.text, q2.text);
                        break;
                    }
                }
            }

            // 3. Nếu không trùng trong file, kiểm tra với database
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
                
                if max_db_similarity > threshold {
                    is_similar = true;
                    similarity_score = max_db_similarity;
                    similarity_type = "database"; // Trùng với database
                    similar_to = format!("Trùng với câu hỏi trong database có độ tương đồng {:.2}%", max_db_similarity * 100.0);
                }
            }
        }

        // Tạo mảng câu trả lời đúng định dạng (với a., b., c.,...)
        let formatted_answers: Vec<String> = q1.answers.iter()
            .enumerate()
            .filter_map(|(i, ans)| {
                // Loại bỏ các đáp án không có nội dung
                let trimmed = ans.trim();
                if trimmed.is_empty() {
                    return None;
                }

                // Kiểm tra nếu đáp án chỉ là chữ cái (như "b", "c", ...)
                let letter = char::from(b'a' + (i as u8 % 26));
                let letter_prefix = format!("{}. ", letter);
                
                // Nếu đáp án trống hoặc chỉ chứa ký tự của nó (b. b, c. c, v.v.) thì bỏ qua
                if trimmed == letter.to_string() || 
                   trimmed == format!("{}.", letter) ||
                   trimmed == format!("{}. {}", letter, letter) ||
                   trimmed == format!("{}. {}.", letter, letter) {
                    return None;
                }
                
                // Trả về đáp án có format
                Some(format!("{}. {}", letter, trimmed))
            })
            .collect();

        // Xử lý đáp án đúng
        let docx_answer: String;
        if q1.correct_answers.len() == 1 {
            // Nếu chỉ có 1 đáp án đúng
            docx_answer = q1.correct_answers[0].clone();
        } else {
            // Nếu có nhiều đáp án đúng
            docx_answer = q1.correct_answers.join(", ");
        }

        // Đảm bảo correct_answer_keys luôn là chữ thường
        let correct_answer_keys: Vec<String> = q1.correct_answer_keys
            .iter()
            .map(|key| key.to_lowercase())
            .collect();

        // Thêm dữ liệu câu hỏi vào kết quả
        let item = serde_json::json!({
            "id": q1.id,
            "docx_question": q1.text,
            "docx_answer": docx_answer,
            "similarity_score": format!("{:.0}%", similarity_score * 100.0),
            "is_similar": is_similar,
            "answers": formatted_answers,
            "correct_answer_keys": correct_answer_keys,
            "correct_answers": q1.correct_answers,
            "similarity_type": similarity_type
        });

        result_items.push(item);
    }
    
    // Tạo kết quả cuối cùng
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

// Hàm main chính cho ứng dụng Tauri
#[cfg(not(feature = "test_fill_format"))]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_docx, process_docx, fill_format_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Hàm main riêng để kiểm thử fill_format_check
#[cfg(feature = "test_fill_format")]
fn main() {
    use std::env;
    use std::fs;
    use std::io;
    use std::process;
    use serde_json::Value;
    
    println!("=== CHẾ ĐỘ KIỂM THỬ FILL_FORMAT_CHECK ===");
    
    // Lấy đường dẫn file từ tham số dòng lệnh hoặc yêu cầu nhập
    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("Nhập đường dẫn đến file DOCX để kiểm tra:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Không thể đọc input");
        input.trim().to_string()
    };
    
    // Kiểm tra file tồn tại
    if !fs::metadata(&file_path).is_ok() {
        eprintln!("Lỗi: File không tồn tại: {}", file_path);
        process::exit(1);
    }
    
    println!("Đang kiểm tra file: {}", file_path);
    
    // Đọc nội dung file
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
            
            // Parse JSON để định dạng lại output
            let result: Value = serde_json::from_str(&json_str).unwrap_or_else(|e| {
                eprintln!("Lỗi phân tích JSON: {}", e);
                process::exit(1);
            });
            
            // In thông tin tổng quan
            println!("Tổng số câu hỏi: {}", result["similarities"].as_array().unwrap_or(&Vec::new()).len());
            println!("Số câu hỏi trong DB: {}", result["db_count"].as_u64().unwrap_or(0));
            
            // In thông tin cho từng câu hỏi, với định dạng tốt hơn
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
                    
                    // In thông tin về trùng lặp
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