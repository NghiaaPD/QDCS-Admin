use docx_rust::document::{BodyContent, ParagraphContent, RunContent, TableCellContent, TableRowContent};
use docx_rust::DocxFile;

pub struct QuestionData {
    pub qn: String,
    pub question: String,
    pub options: Vec<String>,
    pub answer: String,
    pub mark: String,
    pub unit: String,
    pub lo: String,
    pub mix_choices: String,
    pub creator_reviewer: String,
    pub editor: String,
    pub reference: String,
}

pub fn extract_questions_from_docx(file_path: &str) -> Result<Vec<QuestionData>, Box<dyn std::error::Error>> {
    // Đọc và parse document
    let doc_file = DocxFile::from_file(file_path)?;
    let doc = doc_file.parse()?;
    
    let mut questions = Vec::new();
    
    // Lặp qua các phần tử trong body để tìm các bảng
    for element in &doc.document.body.content {
        if let BodyContent::Table(table) = element {
            let mut current_question = QuestionData {
                qn: String::new(),
                question: String::new(),
                options: Vec::new(),
                answer: String::new(),
                mark: String::new(),
                unit: String::new(),
                lo: String::new(),
                mix_choices: String::new(),
                creator_reviewer: String::new(),
                editor: String::new(),
                reference: String::new(),
            };

            // Lặp qua các hàng trong bảng
            for row in &table.rows {
                if row.cells.len() < 2 {
                    continue;
                }

                // Điều chỉnh kiểu dữ liệu - truyền đúng kiểu TableRowContent
                let key = extract_text_from_row_cell(&row.cells[0]).trim().to_string();
                let value = extract_text_from_row_cell(&row.cells[1]).trim().to_string();

                match key.as_str() {
                    "QN=" => current_question.qn = value,
                    "Which of the following best describes an array?" => current_question.question = value,
                    "a." | "b." | "c." | "d." => {
                        if !value.is_empty() {
                            current_question.options.push(value);
                        }
                    }
                    "ANSWER:" => current_question.answer = value,
                    "MARK:" => current_question.mark = value,
                    "UNIT:" => current_question.unit = value,
                    "LO:" => current_question.lo = value,
                    "MIX CHOICES:" => current_question.mix_choices = value,
                    "CREATOR-REVIEWER:" => current_question.creator_reviewer = value,
                    "EDITOR:" => current_question.editor = value,
                    "REFERENCE:" => current_question.reference = value,
                    _ => {}
                }
            }

            questions.push(current_question);
        }
    }

    Ok(questions)
}

// Sửa lại tên hàm để tránh nhầm lẫn và sửa irrefutable if let pattern
fn extract_text_from_row_cell(cell: &TableRowContent) -> String {
    match cell {
        TableRowContent::TableCell(cell_data) => {
            let mut text = String::new();
            
            for content in &cell_data.content {
                // Sửa "if let" thành "let" vì TableCellContent chỉ có variant Paragraph
                let TableCellContent::Paragraph(p) = content;
                for run_content in &p.content {
                    if let ParagraphContent::Run(r) = run_content {
                        for text_content in &r.content {
                            if let RunContent::Text(t) = text_content {
                                text.push_str(&t.text);
                            }
                        }
                    }
                }
            }
            
            text
        },
        _ => String::new()
    }
}