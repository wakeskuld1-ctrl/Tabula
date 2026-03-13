
print("Starting script...")
import requests
import json
import sys

def check_tables():
    print("Requesting REGISTERED tables...")
    url = "http://localhost:3000/api/tables"
    try:
        response = requests.get(url)
        print(f"Response status: {response.status_code}")
        data = response.json()
        
        tables = []
        if isinstance(data, list):
            tables = data
        elif isinstance(data, dict):
            if "tables" in data:
                tables = data["tables"]
            elif "data" in data:
                tables = data["data"]
        
        print(f"Registered tables count: {len(tables)}")
        
        for table in tables:
            t_name = table.get("table_name", "")
            schema = table.get("schema_name", "")
            catalog = table.get("catalog_name", "")
            print(f"RAW: catalog='{catalog}', schema='{schema}', table='{t_name}'")

    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    check_tables()
