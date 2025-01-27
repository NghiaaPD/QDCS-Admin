use pyo3::prelude::*;
use pyo3::types::PyList;

pub fn get_embeddings(texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
    Python::with_gil(|py| {
        println!("Downloading and loading models...");

        let module = PyModule::from_code_bound(
            py,
            include_str!("../../huggingface.py"),
            "huggingface.py",
            "huggingface.py",
        )?;

        let texts_list = PyList::new_bound(py, &texts);
        
        let embeddings = module
            .getattr("get_embeddings")?
            .call1((texts_list,))?;
        
        let embeddings: Vec<Vec<f32>> = embeddings
            .extract()?;

        Ok(embeddings)
    })
}

fn main() -> anyhow::Result<()> {
    // Debug Python path
    println!("PYTHONHOME: {:?}", std::env::var("PYTHONHOME"));
    println!("PYTHONPATH: {:?}", std::env::var("PYTHONPATH"));
    println!("Current dir: {:?}", std::env::current_dir()?);
    
    // Tạo vài câu test
    let texts = vec![
        "Xin chào thế giới".to_string(),
        "Hôm nay là một ngày đẹp trời".to_string(),
        "Tôi đang học lập trình Rust".to_string(),
    ];

    // Gọi hàm get_embeddings
    let embeddings = get_embeddings(texts)?;

    // In kết quả
    println!("Số lượng embeddings: {}", embeddings.len());
    println!("Kích thước mỗi embedding: {}", embeddings[0].len());
    
    // In ra vài giá trị đầu tiên của embedding đầu tiên
    println!("Một số giá trị của embedding đầu tiên:");
    for i in 0..5 {
        print!("{:.6} ", embeddings[0][i]);
    }
    println!();

    Ok(())
}