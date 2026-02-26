use rand::Rng;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
struct Order {
    id: u64,
    transaction_code: String,
    amount: u32,
    status: &'static str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let start_time = Instant::now();
    let row_count = 100_000;

    // Define output path relative to the crate root or current working directory
    // Assuming running from project root or crate root.
    // Let's target the same 'data' directory as the python script
    let output_dir = Path::new("federated_query_engine/data");
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }
    let output_path = output_dir.join("test_100k_orders.csv");

    println!("Generating {} rows to {:?}...", row_count, output_path);

    let mut wtr = csv::Writer::from_path(&output_path)?;
    let mut rng = rand::rng();

    let statuses = ["pending", "completed", "failed"];

    for i in 1..=row_count {
        let status = statuses[rng.random_range(0..3)];

        let order = Order {
            id: i,
            transaction_code: format!("TXN{}", i),
            amount: rng.random_range(10..1000),
            status,
        };

        wtr.serialize(order)?;
    }

    wtr.flush()?;

    let duration = start_time.elapsed();
    println!("Done! Generated {} rows in {:.2?}.", row_count, duration);
    println!(
        "File size: {:.2} MB",
        fs::metadata(&output_path)?.len() as f64 / 1024.0 / 1024.0
    );

    Ok(())
}
