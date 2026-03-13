import requests
import os
import shutil
import time
import json

BASE_URL = "http://localhost:3000"

def wait_for_server():
    print("Waiting for server...")
    for _ in range(30):
        try:
            requests.get(f"{BASE_URL}/health")
            print("Server is up!")
            return True
        except requests.exceptions.ConnectionError:
            time.sleep(1)
    print("Server failed to start.")
    return False

def test_csv_upload():
    print("\nTesting CSV Upload...")
    filename = "smoke_test_orders.csv"
    content = "id,product,quantity\n1,Apple,10\n2,Banana,20\n"
    
    with open(filename, "w") as f:
        f.write(content)
        
    try:
        with open(filename, "rb") as f:
            files = {'file': (filename, f, 'text/csv')}
            response = requests.post(f"{BASE_URL}/api/upload", files=files)
            
        print(f"Upload Status: {response.status_code}")
        print(f"Upload Response: {response.text}")
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        if data.get("status") != "success":
            return False
            
        # Verify Query
        print("Verifying Query...")
        table_name = "smoke_test_orders" # from filename stem
        sql = f"SELECT * FROM {table_name} ORDER BY id"
        
        query_resp = requests.post(f"{BASE_URL}/api/execute", json={"sql": sql})
        print(f"Query Status: {query_resp.status_code}")
        print(f"Query Response: {query_resp.text}")
        
        q_data = query_resp.json()
        rows = q_data.get("rows", [])
        if len(rows) != 2:
            print(f"Expected 2 rows, got {len(rows)}")
            return False
            
        if rows[0][1] != "Apple":
            print(f"Expected Apple, got {rows[0][1]}")
            return False
            
        print("CSV Upload Test Passed!")
        return True
        
    finally:
        if os.path.exists(filename):
            os.remove(filename)

def test_excel_upload():
    print("\nTesting Excel Upload...")
    # Create a dummy excel file if possible, or skip if no libs
    # We can try to use the existing users.xlsx if it exists
    src_excel = "data/users.xlsx"
    if not os.path.exists(src_excel):
        print("users.xlsx not found, skipping Excel test")
        return True
        
    filename = "smoke_test_users.xlsx"
    shutil.copy(src_excel, filename)
    
    try:
        with open(filename, "rb") as f:
            files = {'file': (filename, f, 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet')}
            response = requests.post(f"{BASE_URL}/api/upload", files=files)
            
        print(f"Upload Status: {response.status_code}")
        print(f"Upload Response: {response.text}")
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        if data.get("status") != "success":
            return False
            
        # Verify Query
        print("Verifying Query...")
        table_name = "smoke_test_users"
        sql = f"SELECT * FROM {table_name} LIMIT 1"
        
        query_resp = requests.post(f"{BASE_URL}/api/execute", json={"sql": sql})
        print(f"Query Status: {query_resp.status_code}")
        print(f"Query Response: {query_resp.text}")
        
        q_data = query_resp.json()
        if "error" in q_data and q_data["error"]:
             print(f"Query Error: {q_data['error']}")
             return False
             
        print("Excel Upload Test Passed!")
        return True
        
    finally:
        if os.path.exists(filename):
            os.remove(filename)

if __name__ == "__main__":
    if wait_for_server():
        csv_ok = test_csv_upload()
        excel_ok = test_excel_upload()
        
        if csv_ok and excel_ok:
            print("\nALL SMOKE TESTS PASSED")
            exit(0)
        else:
            print("\nSOME TESTS FAILED")
            exit(1)
    else:
        exit(1)
