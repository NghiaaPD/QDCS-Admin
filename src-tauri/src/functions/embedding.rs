// use rust_bert::pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType};

// pub fn check_duplicate_questions(text1: &str, text2: &str) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
//     let model = SentenceEmbeddingsBuilder::remote(
//         SentenceEmbeddingsModelType::AllMiniLmL12V2
//     ).create_model()?;
    
//     let sentences = [text1, text2];
//     let output = model.encode(&sentences)?;
//     Ok(output)
// }


