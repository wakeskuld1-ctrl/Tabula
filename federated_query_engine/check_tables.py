import requests
import json

try:
    response = requests.get('http://localhost:3000/api/tables')
    data = response.json()
    
    if data['status'] == 'ok':
        tables = data['tables']
        print(f"Total tables: {len(tables)}")
        
        yashan_tables = [t for t in tables if t.get('source_type') == 'yashandb']
        print(f"Total YashanDB tables: {len(yashan_tables)}")
        
        warehouse_tables = [t for t in yashan_tables if 'warehouse' in t['table_name'].lower()]
        
        if warehouse_tables:
            print("Found warehouse tables:")
            for t in warehouse_tables:
                print(f" - {t['table_name']} (Schema: {t.get('schema_name')})")
        else:
            print("No warehouse tables found in YashanDB sources.")
            
            # Print first 10 YashanDB tables to check schema format
            print("First 10 YashanDB tables:")
            for t in yashan_tables[:10]:
                print(f" - {t['table_name']} (Schema: {t.get('schema_name')})")
                
    else:
        print(f"API Error: {data}")
        
except Exception as e:
    print(f"Request failed: {e}")
