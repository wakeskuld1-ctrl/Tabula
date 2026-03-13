
import requests
import json
import sys

def check_tables():
    # Connection ID from check_conns.py: conn_1770370744493 (Oracle TPCC)
    url = "http://localhost:3000/api/connections/conn_1770370744493/tables"
    try:
        response = requests.get(url)
        if response.status_code != 200:
            print(f"Error: Status code {response.status_code}")
            print(response.text)
            return

        response_data = response.json()
        print(f"DEBUG: Response data type: {type(response_data)}")
        print(f"DEBUG: Response keys: {list(response_data.keys()) if isinstance(response_data, dict) else 'Not a dict'}")
        
        if isinstance(response_data, dict) and "tables" in response_data:
            tables = response_data["tables"]
        else:
            print(f"DEBUG: Message: {response_data.get('message', 'No message')}")
            tables = [] # Empty list to avoid error later
            
        print(f"Total tables found: {len(tables)}")
        
        tpcc_tables = ["WAREHOUSE", "DISTRICT", "CUSTOMER", "HISTORY", "ORDER", "NEW_ORDER", "ITEM", "STOCK"]
        found_tpcc = []
        
        # Check for TPCC tables (case insensitive matching)
        for table in tables:
            # Table structure is usually {"owner": "...", "table_name": "..."} or similar based on previous output
            # But the API returns just a list of tables, maybe with schema prefix?
            # Let's inspect the first item to know the structure
            if not isinstance(table, dict):
                 # It might be a list of strings if it's just table names
                 t_name = str(table)
            else:
                 t_name = table.get("table_name", "") or table.get("name", "")
                 owner = table.get("owner", "") or table.get("schema", "")
                 if owner:
                     t_name = f"{owner}.{t_name}"

            for tpcc in tpcc_tables:
                if tpcc.lower() in t_name.lower():
                    found_tpcc.append(t_name)
        
        if found_tpcc:
            print("Found TPCC tables:")
            for t in found_tpcc:
                print(f" - {t}")
        else:
            print("No TPCC tables found.")
            
        # Print first 10 tables to verify structure
        print("\nFirst 10 tables:")
        for t in tables[:10]:
            print(t)

    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    check_tables()
