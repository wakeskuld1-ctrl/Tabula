#[cfg(test)]
mod tests {
    use datafusion::prelude::*;
    // use std::sync::Arc;
    use crate::datasources::yashandb::get_cache_filename;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_consistency_check_logic() {
        // 1. Setup Context
        let ctx = SessionContext::new();

        // 2. Create Dummy Data (CSV)
        let data1 = "id,name\n1,Alice\n2,Bob";
        let data2 = "id,name\n1,Alice\n2,Bob";
        let data3 = "id,name\n1,Alice\n3,Charlie";

        let path1 = "test_table1.csv";
        let path2 = "test_table2.csv";
        let path3 = "test_table3.csv";

        let mut f1 = File::create(path1).unwrap();
        f1.write_all(data1.as_bytes()).unwrap();

        let mut f2 = File::create(path2).unwrap();
        f2.write_all(data2.as_bytes()).unwrap();

        let mut f3 = File::create(path3).unwrap();
        f3.write_all(data3.as_bytes()).unwrap();

        // 3. Register Tables
        ctx.register_csv("table1", path1, CsvReadOptions::new())
            .await
            .unwrap();
        ctx.register_csv("table2", path2, CsvReadOptions::new())
            .await
            .unwrap();
        ctx.register_csv("table3", path3, CsvReadOptions::new())
            .await
            .unwrap();

        // 4. Test Identical Tables (table1 vs table2)
        // Logic from link_identical_tables
        let check_sql = "SELECT count(*) FROM (SELECT * FROM table1 EXCEPT SELECT * FROM table2)";
        let df = ctx.sql(check_sql).await.unwrap();
        let batches = df.collect().await.unwrap();
        let count_val = batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::Int64Array>()
            .map(|arr| arr.value(0))
            .unwrap_or(-1);

        assert_eq!(count_val, 0, "Identical tables should have 0 differences");

        // 5. Test Different Tables (table1 vs table3)
        let check_sql_diff =
            "SELECT count(*) FROM (SELECT * FROM table1 EXCEPT SELECT * FROM table3)";
        let df_diff = ctx.sql(check_sql_diff).await.unwrap();
        let batches_diff = df_diff.collect().await.unwrap();
        let count_val_diff = batches_diff[0]
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::Int64Array>()
            .map(|arr| arr.value(0))
            .unwrap_or(-1);

        // Note: table1 has (1, Alice), (2, Bob). table3 has (1, Alice), (3, Charlie).
        // table1 EXCEPT table3 => (2, Bob). Count should be 1.
        assert_eq!(
            count_val_diff, 1,
            "Different tables should show differences"
        );

        // 6. Test Cache Filename Generation
        let table_name = "MyTable";
        let conn_str = "Driver=YashanDB;Server=127.0.0.1;Port=1234;";
        let filename = get_cache_filename(table_name, conn_str);
        // Implementation does not force lowercase currently
        assert!(
            filename.starts_with("MyTable_"),
            "Filename should start with table name"
        );
        assert!(
            filename.ends_with(".parquet"),
            "Filename should end with .parquet"
        );

        // Cleanup
        std::fs::remove_file(path1).unwrap();
        std::fs::remove_file(path2).unwrap();
        std::fs::remove_file(path3).unwrap();
    }
}
