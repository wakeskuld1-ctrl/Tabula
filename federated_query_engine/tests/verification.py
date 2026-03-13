import requests
import json
import time

BASE_URL = "http://localhost:3000"

def test_health():
    try:
        response = requests.get(f"{BASE_URL}/health")
        # print(f"Health Check: {response.status_code}")
        return response.status_code == 200
    except Exception as e:
        # print(f"Health check failed: {e}")
        return False

def test_query(sql, expected_rows=None):
    print(f"\nExecuting SQL: {sql}")
    try:
        response = requests.post(f"{BASE_URL}/api/execute", json={"sql": sql})
        if response.status_code != 200:
            print(f"Query failed with {response.status_code}: {response.text}")
            return False
        
        data = response.json()
        print("Result Preview:")
        
        if "rows" in data and isinstance(data["rows"], list):
            columns = data.get("columns", [])
            print(f"Columns: {columns}")
            for row in data["rows"][:3]:
                print(row)
            print(f"Total rows: {len(data['rows'])}")
            
            if expected_rows is not None:
                if len(data["rows"]) == expected_rows:
                    print("✅ Row count matches expectation.")
                else:
                    print(f"❌ Row count mismatch. Expected {expected_rows}, got {len(data['rows'])}")
                    return False
        else:
            print("No data returned or unexpected format.")
            print(data)
            
        return True
    except Exception as e:
        print(f"Query execution error: {e}")
        return False

def run_tests():
    print("Waiting for server to be ready...")
    for _ in range(10):
        if test_health():
            print("Server is ready!")
            break
        time.sleep(2)
    else:
        print("Server not reachable. Exiting.")
        return

    # Test 1: Query CSV (Orders)
    print("\n--- Test 1: CSV Query (Orders) ---")
    test_query("SELECT * FROM orders LIMIT 5")

    # Test 2: Query Excel (Users)
    print("\n--- Test 2: Excel Query (Users) ---")
    test_query("SELECT * FROM users LIMIT 5")

    # Test 3: Join CSV and Excel
    print("\n--- Test 3: Federated Join (Orders + Users) ---")
    sql = """
    SELECT 
        o.order_id, 
        o.amount, 
        u.username, 
        u.age 
    FROM orders o 
    JOIN users u ON o.user_id = u.user_id 
    ORDER BY o.amount DESC 
    LIMIT 5
    """
    test_query(sql)

if __name__ == "__main__":
    run_tests()
