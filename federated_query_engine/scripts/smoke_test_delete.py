import requests
import json
import sys

BASE_URL = "http://127.0.0.1:3000/api"

def log(msg):
    print(f"[TEST] {msg}")

def test_delete_non_existent_table():
    log("Testing delete non-existent table...")
    table_name = "non_existent_table_12345"
    resp = requests.post(f"{BASE_URL}/delete_table", json={"table_name": table_name})
    
    log(f"Status Code: {resp.status_code}")
    log(f"Response Body: {resp.text}")
    
    try:
        data = resp.json()
        if data.get("status") == "error" and "Table not found" in data.get("message", ""):
            log("PASS: Correctly returned JSON error for non-existent table.")
        else:
            log("FAIL: Unexpected response content.")
            sys.exit(1)
    except Exception as e:
        log(f"FAIL: Failed to parse JSON: {e}")
        sys.exit(1)

def test_invalid_json_body():
    log("Testing invalid JSON body...")
    # Sending invalid JSON (malformed)
    # requests library makes it hard to send malformed JSON if using `json` param.
    # We use `data` with string.
    resp = requests.post(
        f"{BASE_URL}/delete_table", 
        data="{ 'table_name': 'broken ", 
        headers={"Content-Type": "application/json"}
    )
    
    log(f"Status Code: {resp.status_code}")
    log(f"Response Body: {resp.text}")
    
    try:
        data = resp.json()
        if data.get("status") == "error" and data.get("code") == "JSON_PARSE_ERROR":
            log("PASS: Correctly returned JSON error for invalid JSON body.")
        else:
            log("FAIL: Unexpected response content.")
            sys.exit(1)
    except Exception as e:
        log(f"FAIL: Failed to parse JSON: {e}")
        sys.exit(1)

def test_invalid_content_type():
    log("Testing invalid Content-Type...")
    # Sending text/plain
    resp = requests.post(
        f"{BASE_URL}/delete_table", 
        data=json.dumps({"table_name": "whatever"}), 
        headers={"Content-Type": "text/plain"}
    )
    
    log(f"Status Code: {resp.status_code}")
    log(f"Response Body: {resp.text}")
    
    try:
        data = resp.json()
        # Axum JsonRejection for MissingJsonContentType usually returns 415 or 400
        # Our handler catches JsonRejection, so it should be our JSON error.
        if data.get("status") == "error" and data.get("code") == "JSON_PARSE_ERROR":
            log("PASS: Correctly returned JSON error for invalid Content-Type.")
        else:
            log("FAIL: Unexpected response content.")
            sys.exit(1)
    except Exception as e:
        log(f"FAIL: Failed to parse JSON: {e}")
        sys.exit(1)

if __name__ == "__main__":
    try:
        test_delete_non_existent_table()
        print("-" * 20)
        test_invalid_json_body()
        print("-" * 20)
        test_invalid_content_type()
        print("-" * 20)
        print("ALL TESTS PASSED")
    except Exception as e:
        print(f"TEST SUITE FAILED: {e}")
        sys.exit(1)
