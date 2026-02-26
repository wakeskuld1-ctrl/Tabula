import requests
import json
import time

BASE_URL = "http://127.0.0.1:3000"

def log(msg):
    print(f"[TEST] {msg}")

def check_tables():
    try:
        res = requests.get(f"{BASE_URL}/api/tables")
        if res.status_code == 200:
            tables = res.json().get("tables", [])
            log(f"Registered tables: {tables}")
            return tables
        else:
            log(f"Failed to list tables: {res.text}")
            return []
    except Exception as e:
        log(f"Error listing tables: {e}")
        return []

def run_query(sql, description):
    log(f"Running: {description} -> {sql}")
    try:
        res = requests.post(f"{BASE_URL}/api/execute", json={"sql": sql})
        if res.status_code == 200:
            data = res.json()
            if data.get("error"):
                log(f"FAILED (App Error): {data['error']}")
                return False
            else:
                rows = len(data.get("rows", []))
                cols = len(data.get("columns", []))
                col_names = data.get("columns", [])
                first_row = data.get("rows", [])[0] if rows > 0 else []
                log(f"SUCCESS: {rows} rows, {cols} columns")
                log(f"Columns: {col_names}")
                if first_row:
                    log(f"First Row: {first_row}")
                return True
        else:
            log(f"FAILED (HTTP {res.status_code}): {res.text}")
            return False
    except Exception as e:
        log(f"FAILED (Exception): {e}")
        return False

def explain_query(sql, description):
    log(f"Explaining: {description} -> {sql}")
    try:
        res = requests.post(f"{BASE_URL}/api/plan", json={"sql": sql})
        if res.status_code == 200:
            data = res.json()
            if "Physical Plan Error" in data.get("physical_plan_text", ""):
                 log(f"FAILED (Plan Error): {data['physical_plan_text']}")
                 return False
            else:
                 log(f"SUCCESS: Got plan")
                 return True
        else:
            log(f"FAILED (HTTP {res.status_code}): {res.text}")
            return False
    except Exception as e:
        log(f"FAILED (Exception): {e}")
        return False

def main():
    log("Starting verification loop...")
    tables = check_tables()
    
    # 1. Basic CSV
    run_query("SELECT * FROM orders LIMIT 5", "Basic CSV Select")
    
    # 2. Basic Excel
    run_query("SELECT * FROM users LIMIT 5", "Basic Excel Select")
    
    # 3. Complex Name (if exists)
    complex_tables = [t for t in tables if "PingCode" in t]
    if complex_tables:
        t = complex_tables[0]
        # Try without quotes (should be handled by rewriter)
        run_query(f"SELECT * FROM {t} LIMIT 1", "Complex Name (Unquoted)")
        # Try with quotes
        run_query(f'SELECT * FROM "{t}" LIMIT 1', "Complex Name (Quoted)")
    
    # 4. Field Resolution
    run_query("SELECT user_id FROM users LIMIT 1", "Specific Field (Excel)")
    run_query("SELECT order_id FROM orders LIMIT 1", "Specific Field (CSV)")
    run_query("SELECT ORDER_ID FROM orders LIMIT 1", "Specific Field (CSV Case-Insensitive)")
    
    # 5. Reproduce Panic (if test_table exists)
    if "test_table" in tables:
        run_query("SELECT a.email FROM test_table a LIMIT 1", "Panic Reproduction")
    
    # 6. 用户场景验证: JOIN 查询检查
    # 首先检查各个表的数据是否正常
    run_query("SELECT * FROM test_table LIMIT 5", "Inspect test_table")
    run_query("SELECT * FROM test_data_1000 LIMIT 5", "Inspect test_data_1000")
    
    # 用户反馈的失败查询复现
    run_query("SELECT a.email FROM test_table a left join test_data_1000 b on a.id=b.id LIMIT 100", "User JOIN Query Failure Repro")
    
    # 8. 用户反馈 SUM(salary) 无值复现
    run_query("SELECT a.age, sum(a.salary) FROM test_data_1000 a left join test_table b on a.id=b.id group by a.age LIMIT 5", "User SUM(salary) Query Repro")

    # 调试元数据
    run_query("SELECT * FROM sys_metadata WHERE table_name = 'test_table'", "Check test_table metadata")
    run_query("SELECT * FROM sys_metadata WHERE table_name = 'test_data_1000'", "Check test_data_1000 metadata")

    # 调试数据量和 JOIN
    run_query("SELECT count(*) FROM test_table", "Count test_table")
    run_query("SELECT count(*) FROM test_data_1000", "Count test_data_1000")
    run_query("SELECT count(*) FROM test_table a JOIN test_data_1000 b ON a.id = b.id", "Inner Join Count")
    
    # Inspect schema types
    print("\n[TEST] Checking Schema Types...")
    try:
        run_query("SELECT arrow_typeof(salary) FROM test_data_1000 LIMIT 1", "test_data_1000 salary type")
        run_query("SELECT arrow_typeof(id) FROM test_data_1000 LIMIT 1", "test_data_1000 id type")
        run_query("SELECT arrow_typeof(salary) FROM test_table LIMIT 1", "test_table salary type")
        run_query("SELECT arrow_typeof(id) FROM test_table LIMIT 1", "test_table id type")
    except Exception as e:
        print(f"[TEST] Schema check failed: {e}")

    # Run the problem query again
    problem_sql = "SELECT a.age, sum(b.salary) FROM test_table a left join test_data_1000 b on a.id=b.id group by a.age LIMIT 5"
    run_query(problem_sql, "Problem Query: Sum(b.salary)")
    explain_query(problem_sql, "Problem Query Plan")

    # 检查字段类型 (通过简单的 SELECT 获取 schema 信息 - 虽然这个脚本只打印值，但 plan 会显示类型)
    res = requests.post(f"{BASE_URL}/api/plan", json={"sql": "SELECT id, salary FROM test_table LIMIT 1"})
    log(f"Plan test_table types:\n{res.json().get('physical_plan_text', '')}")

    res = requests.post(f"{BASE_URL}/api/plan", json={"sql": "SELECT id, salary FROM test_data_1000 LIMIT 1"})
    log(f"Plan test_data_1000 types:\n{res.json().get('physical_plan_text', '')}")

    # 7. Check Plan API and Pushdown
    log("Checking Plan API (CSV)...")
    try:
        # Assuming test_data_1000 is a large table
        res = requests.post(f"{BASE_URL}/api/plan", json={"sql": "SELECT * FROM test_data_1000 WHERE id = 1"})
        if res.status_code == 200:
            data = res.json()
            plan_text = data.get("physical_plan_text", "")
            est_rows = data.get("estimated_rows")
            est_bytes = data.get("estimated_bytes")
            log(f"CSV Plan Stats: Rows={est_rows}, Bytes={est_bytes}")
            log(f"Plan Text Preview:\n{plan_text[:500]}")
            if "filters=" in plan_text or "FilterExec" in plan_text:
                log("SUCCESS: Plan generated (filters or FilterExec found)")
            else:
                log("WARNING: Plan generated but no filters/FilterExec found")
        else:
            log(f"FAILED to get plan: {res.text}")
    except Exception as e:
        log(f"FAILED to call plan API: {e}")

    log("Checking Plan API (SQLite)...")
    try:
        res = requests.post(f"{BASE_URL}/api/plan", json={"sql": "SELECT * FROM test_table WHERE id = 1"})
        if res.status_code == 200:
            data = res.json()
            plan_text = data.get("physical_plan_text", "")
            est_rows = data.get("estimated_rows")
            est_bytes = data.get("estimated_bytes")
            log(f"SQLite Plan Stats: Rows={est_rows}, Bytes={est_bytes}")
            log(f"SQLite Plan Text:\n{plan_text}")
        else:
            log(f"FAILED to get plan: {res.text}")
    except Exception as e:
        log(f"FAILED to call plan API: {e}")

if __name__ == "__main__":
    main()
