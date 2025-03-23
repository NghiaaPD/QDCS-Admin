pub fn calculate_similarity_score(question_similarity: f32, answer_similarity: f32) -> f32 {
    if question_similarity >= 0.5 && answer_similarity >= 0.5 {
        (question_similarity + answer_similarity) / 2.0
    } else {
        question_similarity.min(answer_similarity)
    }
}

