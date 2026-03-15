use rusqlite::{Connection, Result};
use std::path::Path;

fn main() -> Result<()> {
    let paths = ["federated_query_engine/metadata.db", "metadata.db"];

    let mut db_path = "";
    for p in &paths {
        if Path::new(p).exists() {
            db_path = p;
            break;
        }
    }

    if db_path.is_empty() {
        // Fallback: try absolute path if known
        if Path::new("d:/Rust/metadata/federated_query_engine/metadata.db").exists() {
            db_path = "d:/Rust/metadata/federated_query_engine/metadata.db";
        } else {
            eprintln!("Could not find metadata.db. Checked paths: {:?}", paths);
            return Ok(());
        }
    }

    println!("Opening database at: {}", db_path);
    let conn = Connection::open(db_path)?;

    // List all tables first for debugging
    {
        let mut stmt = conn.prepare("SELECT table_name FROM tables_metadata")?;
        let table_names = stmt.query_map([], |row| row.get::<_, String>(0))?;
        println!("Current tables in metadata:");
        for name in table_names {
            println!(" - {}", name?);
        }
    }

    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM tables_metadata WHERE table_name LIKE 'datafusion.public.%'",
    )?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;

    if count == 0 {
        println!("No dirty tables found (table_name LIKE 'datafusion.public.%').");
    } else {
        println!("Found {} dirty tables. Deleting...", count);
        let deleted = conn.execute(
            "DELETE FROM tables_metadata WHERE table_name LIKE 'datafusion.public.%'",
            [],
        )?;
        println!("Deleted {} rows.", deleted);
    }

    // Also clean specific problematic tables mentioned by user
    println!("Cleaning up PingCode and test_data tables...");
    // Use leading wildcard to catch any variations
    let deleted_pingcode = conn.execute("DELETE FROM tables_metadata WHERE table_name LIKE '%PingCode%' OR table_name LIKE '%test_data%'", [])?;
    println!("Deleted {} PingCode/test_data rows.", deleted_pingcode);

    // Vacuum to reclaim space
    conn.execute("VACUUM", [])?;
    println!("Database vacuumed.");

    Ok(())
}
