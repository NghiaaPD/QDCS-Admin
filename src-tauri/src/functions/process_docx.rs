use docx_rust::DocxFile;
use docx_rust::document::{BodyContent, ParagraphContent, RunContent, TableRowContent, TableCellContent};
use std::io::Cursor;

// Thêm struct để lưu thông tin câu hỏi
#[derive(Debug)]
pub struct Question {
    pub text: String,
    pub correct_answer_text: String,
}

pub fn read_docx_content_from_bytes(bytes: &[u8]) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    // Tạo một cursor từ bytes
    let cursor = Cursor::new(bytes);
    
    // Sử dụng from_reader thay vì from_bytes
    let docx = DocxFile::from_reader(cursor)?;
    let docx = docx.parse()?;
    let mut questions = Vec::new();

    for element in &docx.document.body.content {
        if let BodyContent::Table(table) = element {
            let mut question = Question {
                text: String::new(),
                correct_answer_text: String::new(),
            };

            let mut answer_texts: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            // Đọc nội dung câu hỏi từ ô thứ hai của dòng đầu tiên
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

            // Kiểm tra nội dung câu hỏi
            if question.text.is_empty() {
                return Err("File sai format: Thiếu nội dung câu hỏi".into());
            }

            // Đọc text của các đáp án
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
                
                // Lưu text của các đáp án
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
                
                // Tìm và lưu text của đáp án đúng
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

            questions.push(question);
        }
    }
    
    Ok(questions)
}

