
print("Starting script...")
import requests
import json
import sys

def check_tables():
    print("Requesting tables...")
    url = "http://localhost:3000/api/connections/conn_1770101997508/tables"
    try:
        response = requests.get(url)
        print(f"Response status: {response.status_code}")
        data = response.json()
        print(f"Data type: {type(data)}")
        
        tables = []
        if isinstance(data, list):
            tables = data
        elif isinstance(data, dict):
            print(f"Keys: {list(data.keys())}")
            # Try to find the list of tables
            if "tables" in data:
                tables = data["tables"]
            elif "data" in data:
                tables = data["data"]
            else:
                print("Could not find tables list in dict.")
                # print first key's value type
                first_key = list(data.keys())[0]
                print(f"Value of {first_key}: {type(data[first_key])}")
        
        print(f"Actual tables list count: {len(tables)}")
        
        target_schema = "TPCC"
        target_prefix = "BMSQL"
        
        found = []
        for table in tables:
            t_name = ""
            owner = ""
            if isinstance(table, dict):
                 t_name = table.get("table_name", "") or table.get("name", "")
                 owner = table.get("owner", "") or table.get("schema", "")
            elif isinstance(table, list) and len(table) >= 2:
                 owner = str(table[0])
                 t_name = str(table[1])
            
            if target_schema.lower() in owner.lower() or target_prefix.lower() in t_name.lower():
                found.append(f"{owner}.{t_name}")

        if found:
            print(f"Found {len(found)} target tables:")
            for t in found:
                print(f" - {t}")
        else:
            print("No BMSQL or TPCC tables found.")
            
    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    check_tables()
