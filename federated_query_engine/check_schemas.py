import requests
import json

try:
    response = requests.get('http://localhost:3000/api/tables')
    data = response.json()
    
    if data['status'] == 'ok':
        tables = data['tables']
        yashan_tables = [t for t in tables if t.get('source_type') == 'yashandb']
        
        schemas = set()
        for t in yashan_tables:
            s = t.get('schema_name')
            if s:
                schemas.add(s)
        
        print(f"Total schemas found: {len(schemas)}")
        if 'TPCC' in schemas:
            print("TPCC schema found!")
        else:
            print("TPCC schema NOT found.")
            
        # Print all schemas for debugging
        # for s in sorted(list(schemas)):
        #     print(s)
        
    else:
        print(f"API Error: {data}")
        
except Exception as e:
    print(f"Request failed: {e}")
