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

#[cfg(not(feature = "test_fill_format"))]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_docx, process_docx, fill_format_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(feature = "test_fill_format")]
fn main() {
    use std::env;
    use std::fs;
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

            println!("Tổng số câu hỏi: {}", result["similarities"].as_array().unwrap_or(&Vec::new()).len());
            println!("Số câu hỏi trong DB: {}", result["db_count"].as_u64().unwrap_or(0));

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