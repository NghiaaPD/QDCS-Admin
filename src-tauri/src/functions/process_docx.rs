use anyhow::{anyhow, Result};
use docx_rust::document::{
    BodyContent, ParagraphContent, RunContent, Table, TableCell, TableCellContent, TableRowContent,
};
use docx_rust::DocxFile;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{io::Cursor, sync::LazyLock};

#[derive(Debug)]
pub struct Question {
    pub text: String,
    pub correct_answer_text: String,
    pub question_embedding: Vec<f32>,
    pub answer_embedding: Vec<f32>,
}

impl std::fmt::Display for Question {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Câu hỏi: {}\nĐáp án đúng: {}\nEmbedding câu hỏi: {:?}\nEmbedding đáp án: {:?}",
            self.text, self.correct_answer_text, self.question_embedding, self.answer_embedding
        )
    }
}

static MODEL: LazyLock<TextEmbedding> = LazyLock::new(|| {
    TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
        .expect("Failed to init embedding model")
});

fn get_table_cell_content(cell_data: &TableCell<'_>) -> String {
    cell_data
        .content
        .iter()
        .flat_map(|content| {
            let TableCellContent::Paragraph(paragraph) = content;
            paragraph.content.iter().filter_map(|run| match run {
                ParagraphContent::Run(r) => Some(r),
                _ => None,
            })
        })
        .flat_map(|run| {
            run.content.iter().filter_map(|text| match text {
                RunContent::Text(text) => Some(text.text.as_ref()),
                _ => None,
            })
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_table(table: &Table<'_>) -> Result<Question> {
    let mut question = Question {
        text: String::new(),
        correct_answer_text: String::new(),
        question_embedding: Vec::new(),
        answer_embedding: Vec::new(),
    };

    let mut answer_texts: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    question.text = table
        .rows
        .first()
        .and_then(|first_row| first_row.cells.get(1))
        .and_then(|cell| match cell {
            TableRowContent::TableCell(cell_data) => Some(get_table_cell_content(cell_data)),
            _ => None,
        })
        .ok_or(anyhow!("File sai format: Thiếu nội dung câu hỏi"))?;

    for row in table.rows.iter() {
        let first_cell_text = match &row.cells.first() {
            Some(TableRowContent::TableCell(cell_data)) => get_table_cell_content(cell_data),
            _ => String::new(),
        };

        let first_cell_text = first_cell_text.trim();

        if first_cell_text.len() == 2 && first_cell_text.ends_with('.') {
            let option_key = first_cell_text
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .to_string();
            if let Some(TableRowContent::TableCell(cell_data)) = row.cells.get(1) {
                let answer_text = get_table_cell_content(cell_data);
                answer_texts.insert(option_key, answer_text.trim().to_string());
            }
        }

        if first_cell_text == "ANSWER:" && row.cells.len() > 1 {
            if let Some(TableRowContent::TableCell(cell_data)) = row.cells.get(1) {
                let correct_answer = get_table_cell_content(cell_data);

                if let Some(text) = answer_texts.get(correct_answer.trim()) {
                    question.correct_answer_text = text.clone();
                }
            }
        }
    }

    if !question.text.is_empty() {
        let question_embeddings = MODEL.embed(vec![&question.text], None)?;
        question.question_embedding = question_embeddings
            .into_iter()
            .next()
            .expect("Failed to calculate question embedding");
    }

    if !question.correct_answer_text.is_empty() {
        let answer_embeddings = MODEL.embed(vec![&question.correct_answer_text], None)?;
        question.answer_embedding = answer_embeddings
            .into_iter()
            .next()
            .expect("Failed to calculate answer embedding");
    }

    Ok(question)
}

pub fn read_docx_content_from_bytes(bytes: &[u8]) -> Result<Vec<Question>> {
    let cursor = Cursor::new(bytes);

    let docx = DocxFile::from_reader(cursor)?;
    let docx = docx.parse()?;

    let questions = docx
        .document
        .body
        .content
        .par_iter()
        .filter_map(|element| match element {
            BodyContent::Table(table) => Some(table),
            _ => None,
        })
        .map(|table| parse_table(table))
        .collect::<Result<Vec<_>>>()?;

    println!("\n=== All Questions ===");
    for (i, q) in questions.iter().enumerate() {
        println!("\nQuestion {}:", i + 1);
        println!("{}", q);
    }

    Ok(questions)
}
