use docx_rust::DocxFile;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use docx_rust::document::{BodyContent, ParagraphContent, RunContent, TableRowContent, TableCellContent};
use std::io::Cursor;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct Question {
    pub text: String,
    pub correct_answer_text: String,
    pub question_embedding: Vec<f32>,
    pub answer_embedding: Vec<f32>,
}

impl std::fmt::Display for Question {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Câu hỏi: {}\nĐáp án đúng: {}\nEmbedding câu hỏi: {:?}\nEmbedding đáp án: {:?}", 
            self.text, 
            self.correct_answer_text,
            self.question_embedding,
            self.answer_embedding
        )
    }
}

static MODEL: Lazy<TextEmbedding> = Lazy::new(|| {
    TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
        .expect("Không thể khởi tạo model embedding")
});

pub fn read_docx_content_from_bytes(bytes: &[u8]) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(bytes);
    
    let docx = DocxFile::from_reader(cursor)?;
    let docx = docx.parse()?;
    let mut questions = Vec::new();

    for element in &docx.document.body.content {
        if let BodyContent::Table(table) = element {
            let mut question = Question {
                text: String::new(),
                correct_answer_text: String::new(),
                question_embedding: Vec::new(),
                answer_embedding: Vec::new(),
            };

            let mut answer_texts: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            if let Some(first_row) = table.rows.first() {
                if let Some(TableRowContent::TableCell(cell_data)) = first_row.cells.get(1) {
                    question.text = cell_data.content.iter().fold(String::new(), |acc, content| {
                        let TableCellContent::Paragraph(p) = content;
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
                    }).trim().to_string();
                }
            }

            if question.text.is_empty() {
                return Err("File sai format: Thiếu nội dung câu hỏi".into());
            }

            for row in table.rows.iter() {
                let first_cell_text = match &row.cells.first() {
                    Some(TableRowContent::TableCell(cell_data)) => {
                        cell_data.content.iter().fold(String::new(), |acc, content| {
                            let TableCellContent::Paragraph(p) = content;
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
                        })
                    },
                    _ => String::new()
                };

                let first_cell_text = first_cell_text.trim();

                if first_cell_text.len() == 2 && first_cell_text.ends_with('.') {
                    let option_key = first_cell_text.chars().next().unwrap().to_uppercase().to_string();
                    if let Some(TableRowContent::TableCell(cell_data)) = row.cells.get(1) {
                        let answer_text = cell_data.content.iter().fold(String::new(), |acc, content| {
                            let TableCellContent::Paragraph(p) = content;
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
                        });
                        answer_texts.insert(option_key, answer_text.trim().to_string());
                    }
                }

                if first_cell_text == "ANSWER:" && row.cells.len() > 1 {
                    if let Some(TableRowContent::TableCell(cell_data)) = row.cells.get(1) {
                        let correct_answer = cell_data.content.iter().fold(String::new(), |acc, content| {
                            let TableCellContent::Paragraph(p) = content;
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
                        });
                        
                        if let Some(text) = answer_texts.get(&correct_answer.trim().to_string()) {
                            question.correct_answer_text = text.clone();
                        }
                    }
                }
            }

            if !question.text.is_empty() {
                let question_embeddings = MODEL.embed(vec![question.text.clone()], None)?;
                question.question_embedding = question_embeddings[0].clone();
            }

            if !question.correct_answer_text.is_empty() {
                let answer_embeddings = MODEL.embed(vec![question.correct_answer_text.clone()], None)?;
                question.answer_embedding = answer_embeddings[0].clone();
            }

            questions.push(question);
        }
    }
    
    println!("\n=== All Questions ===");
    for (i, q) in questions.iter().enumerate() {
        println!("\nQuestion {}:", i + 1);
        println!("{}", q);
    }
    
    Ok(questions)
}

