import requests
import json
import time
import sys

BASE_URL = "http://localhost:3000"

def get_oracle_connections():
    try:
        response = requests.get(f"{BASE_URL}/api/connections")
        if response.status_code != 200:
            print(f"Failed to get connections: {response.text}")
            return []
        
        data = response.json()
        connections = []
        if isinstance(data, dict):
            if "connections" in data:
                connections = data["connections"]
            elif "data" in data:
                connections = data["data"]
            else:
                connections = [] 
        elif isinstance(data, list):
            connections = data
            
        oracle_conns = []
        for conn in connections:
            source_type = conn.get("source_type") or conn.get("type")
            if isinstance(conn, dict) and source_type == "oracle":
                oracle_conns.append(conn)
        return oracle_conns
    except Exception as e:
        print(f"Error getting connections: {e}")
        return []

def register_table(conn, table_name):
    config_val = conn.get("config")
    if not config_val:
        print("No config found in connection")
        return False
        
    config = {}
    if isinstance(config_val, dict):
        config = config_val
    elif isinstance(config_val, str):
        try:
            config = json.loads(config_val)
        except:
            print(f"Failed to parse config JSON: {config_val}")
            return False
    else:
        print(f"Unknown config type: {type(config_val)}")
        return False

    payload = {
        "user": config.get("user"),
        "pass": config.get("pass"),
        "host": config.get("host"),
        "port": int(config.get("port")),
        "service": config.get("service"),
        "table_name": table_name,
        "schema": "TPCC" 
    }
    
    try:
        print(f"Registering table {table_name} using connection {conn.get('name')}...")
        response = requests.post(f"{BASE_URL}/api/datasources/oracle/register", json=payload)
        if response.status_code == 200:
            print(f"Registered table {table_name}")
            return True
        else:
            print(f"Failed to register table: {response.text}")
            return False
    except Exception as e:
        print(f"Error registering table: {e}")
        return False

def execute_query(sql):
    payload = {
        "sql": sql,
        "preview": True
    }
    try:
        print(f"Executing SQL: {sql}")
        response = requests.post(f"{BASE_URL}/api/execute", json=payload)
        if response.status_code == 200:
            result = response.json()
            if result.get("status") == "success" or "rows" in result:
                print("Query executed successfully")
                if "rows" in result:
                    print(f"Rows count: {len(result['rows'])}")
                    if len(result['rows']) > 0:
                         print(f"First row: {result['rows'][0]}")
                return True
            else:
                print(f"Query failed: {result}")
                return False
        else:
            print(f"Request failed: {response.text}")
            return False
    except Exception as e:
        print(f"Error executing query: {e}")
        return False

def main():
    print("Waiting for server to start...")
    
    conns = get_oracle_connections()
    if not conns:
        print("No Oracle connections found")
        return
        
    target_conn = None
    # Prefer connection with 'tpcc' in name
    for c in conns:
        if 'tpcc' in c.get('name', '').lower():
            target_conn = c
            break
            
    if not target_conn:
        print("No TPCC oracle connection found, using first available")
        target_conn = conns[0]
        
    print(f"Using connection: {target_conn.get('name')}")

    # Register BMSQL_WAREHOUSE
    if register_table(target_conn, "BMSQL_WAREHOUSE"):
        # Execute query
        print("\n--- Test 1: Unqualified Name (tpcc.BMSQL_WAREHOUSE) ---")
        success1 = execute_query("SELECT * FROM tpcc.BMSQL_WAREHOUSE LIMIT 5")
        
        print("\n--- Test 2: Simple Name (BMSQL_WAREHOUSE) ---")
        success2 = execute_query("SELECT * FROM BMSQL_WAREHOUSE LIMIT 5")

        if success1 or success2:
            print("\nSUCCESS: At least one query format worked!")
        else:
            print("\nFAILURE: Both queries failed.")

if __name__ == "__main__":
    main()
