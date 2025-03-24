use crate::functions::cosine_similarity::calculate_cosine_similarity;
use crate::middleware::fill_format::EMBEDDING_MODEL;
use crate::middleware::fill_format::Question;
use crate::functions::load_accurancy::load_similarity_threshold;
use std::collections::HashMap;

pub fn check_duplicate_answers(answers: &Vec<String>) -> Option<(String, String, f32)> {
    let threshold = match load_similarity_threshold() {
        Ok(t) => t,
        Err(_) => return None,
    };
    
    if answers.len() < 2 {
        return None;
    }
        
    let mut embeddings = Vec::new();
    for ans in answers {
        if ans.len() <= 3 {
            continue;
        }
        
        match EMBEDDING_MODEL.embed(vec![ans], None) {
            Ok(mut emb) => embeddings.push((ans, emb.remove(0))),
            Err(_) => continue,
        }
    }

    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            let similarity = calculate_cosine_similarity(&embeddings[i].1, &embeddings[j].1);
            
            if similarity > threshold {
                return Some((
                    embeddings[i].0.clone(),
                    embeddings[j].0.clone(),
                    similarity
                ));
            }
        }
    }
    
    None
}

// Hàm mới: Kiểm tra đáp án trùng lặp trong cùng một câu hỏi
pub fn check_duplicates_within_question(question: &Question) -> Option<(String, String, f32)> {
    // Tách nội dung các đáp án (bỏ qua phần a., b., c.,...)
    let mut answer_contents: HashMap<String, String> = HashMap::new();
    
    for ans in &question.answers {
        if let Some(pos) = ans.find('.') {
            let key = ans[0..pos].trim().to_string();
            let content = ans[pos+1..].trim().to_string();
            
            if !content.is_empty() {
                for (existing_content, existing_key) in &answer_contents {
                    if content.eq_ignore_ascii_case(existing_content) {
                        return Some((
                            format!("{}.{}", existing_key, existing_content),
                            ans.clone(),
                            1.0
                        ));
                    }
                }
                
                answer_contents.insert(content, key);
            }
        }
    }
    
    let threshold = load_similarity_threshold().unwrap_or(0.6);
    
    let mut embeddings = Vec::new();
    for ans in &question.answers {
        // Tách nội dung đáp án
        if let Some(pos) = ans.find('.') {
            let content = ans[pos+1..].trim();
            if content.len() > 3 {
                match EMBEDDING_MODEL.embed(vec![content], None) {
                    Ok(mut emb) => embeddings.push((ans.clone(), emb.remove(0))),
                    Err(_) => continue,
                }
            }
        }
    }
    
    // So sánh từng cặp embedding
    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            let similarity = calculate_cosine_similarity(&embeddings[i].1, &embeddings[j].1);
            
            if similarity > threshold {
                return Some((
                    embeddings[i].0.clone(),
                    embeddings[j].0.clone(),
                    similarity
                ));
            }
        }
    }
    
    None
}

// Hàm mới: Kiểm tra câu hỏi trùng lặp
#[allow(dead_code)]
pub fn check_duplicate_questions(questions: &[Question]) -> Vec<(usize, usize, f32)> {
    let threshold = load_similarity_threshold().unwrap_or(0.6);
    let mut duplicates = Vec::new();
    let mut question_text_map: HashMap<String, usize> = HashMap::new();
    
    // Kiểm tra trùng text trước
    for (i, q) in questions.iter().enumerate() {
        let normalized_text = q.text.to_lowercase();
        
        if let Some(prev_idx) = question_text_map.get(&normalized_text) {
            duplicates.push((*prev_idx, i, 1.0)); // Trùng hoàn toàn
        } else {
            question_text_map.insert(normalized_text, i);
        }
    }
    
    // Sau đó kiểm tra similarity
    for i in 0..questions.len() {
        for j in (i + 1)..questions.len() {
            // Bỏ qua nếu đã xác định là trùng
            if duplicates.iter().any(|(a, b, _)| (*a == i && *b == j) || (*a == j && *b == i)) {
                continue;
            }
            
            let similarity = calculate_cosine_similarity(
                &questions[i].question_embedding,
                &questions[j].question_embedding
            );
            
            if similarity > threshold {
                duplicates.push((i, j, similarity));
            }
        }
    }
    
    duplicates
}