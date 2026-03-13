
import requests
import json
import sys

def check_conn_tables(conn_id):
    url = f"http://localhost:3000/api/connections/{conn_id}/tables"
    print(f"Querying: {url}")
    try:
        response = requests.get(url)
        # print(f"Raw Response: {response.text}")
        data = response.json()
        
        tables = []
        if isinstance(data, dict) and "data" in data:
            tables = data["data"]
        elif isinstance(data, list):
            tables = data
            
        print(f"Tables found: {len(tables)}")
        
        tpcc_tables = [
            "BMSQL_CONFIG", "BMSQL_CUSTOMER", "BMSQL_DISTRICT", 
            "BMSQL_HISTORY", "BMSQL_ITEM", "BMSQL_NEW_ORDER", 
            "BMSQL_OORDER", "BMSQL_ORDER_LINE", "BMSQL_STOCK", 
            "BMSQL_WAREHOUSE"
        ]
        
        found_count = 0
        for t in tables:
            t_name = t.get('table_name', '').upper()
            s_name = t.get('schema_name', '').upper()
            full_name = f"{s_name}.{t_name}" if s_name else t_name
            
            if "BMSQL" in t_name:
                print(f"Found TPCC Table: {full_name}")
                found_count += 1
                
        print(f"Total TPCC Tables found: {found_count}")

    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    conn_id = "conn_1770111588950"
    if len(sys.argv) > 1:
        conn_id = sys.argv[1]
    check_conn_tables(conn_id)
