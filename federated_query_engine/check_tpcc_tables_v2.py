
import requests
import json
import sys

def check_tables():
    # Connection ID from previous context: conn_1770101997508
    url = "http://localhost:3000/api/connections/conn_1770101997508/tables"
    try:
        response = requests.get(url)
        if response.status_code != 200:
            print(f"Error: Status code {response.status_code}")
            print(response.text)
            return

        data = response.json()
        print(f"Response type: {type(data)}")
        
        if isinstance(data, list):
            print(f"Total items: {len(data)}")
            if len(data) > 0:
                print(f"First item type: {type(data[0])}")
                print(f"First item: {data[0]}")
        elif isinstance(data, dict):
            print(f"Keys: {list(data.keys())}")
            
        # Re-check for TPCC tables assuming it's a list of dicts (the usual case)
        # or maybe the API returns {"tables": [...]}
        
        tables = []
        if isinstance(data, list):
            tables = data
        elif isinstance(data, dict) and "tables" in data:
            tables = data["tables"]
        
        print(f"Processing {len(tables)} tables...")
        
        tpcc_tables = ["WAREHOUSE", "DISTRICT", "CUSTOMER", "HISTORY", "ORDER", "NEW_ORDER", "ITEM", "STOCK"]
        found_count = 0
        
        for table in tables:
            t_name = ""
            owner = ""
            if isinstance(table, dict):
                 t_name = table.get("table_name", "") or table.get("name", "")
                 owner = table.get("owner", "") or table.get("schema", "")
            elif isinstance(table, list) and len(table) >= 2:
                 # Tuple style [owner, table_name, ...]
                 owner = str(table[0])
                 t_name = str(table[1])
            else:
                 t_name = str(table)

            full_name = f"{owner}.{t_name}" if owner else t_name
            
            # Debug print first few
            if found_count < 3 and len(tables) > 0: 
                 # print(f"Sample: {full_name}") 
                 pass

            for tpcc in tpcc_tables:
                if tpcc.lower() in t_name.lower():
                    print(f"FOUND MATCH: {full_name}")
                    found_count += 1
        
        if found_count == 0:
            print("Still no TPCC tables found.")
        else:
            print(f"Total TPCC tables found: {found_count}")

    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    check_tables()
