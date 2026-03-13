import requests
import json
import sys

def check_all():
    # 1. Get all connections
    try:
        print("Starting check...", flush=True)
        resp = requests.get("http://localhost:3000/api/connections")
        print(f"Response status: {resp.status_code}", flush=True)
        data = resp.json()
        print(f"Data type: {type(data)}", flush=True)
        
        conns = []
        if isinstance(data, list):
            print("Data is a list", flush=True)
            conns = data
        elif isinstance(data, dict):
            print("Data is a dict", flush=True)
            if 'data' in data:
                conns = data['data']
            elif 'connections' in data:
                conns = data['connections']
        
        print(f"Connections list type: {type(conns)}", flush=True)
        if len(conns) > 0:
            print(f"First item type: {type(conns[0])}", flush=True)
            print(f"First item: {conns[0]}", flush=True)

        with open('conns_output.txt', 'w', encoding='utf-8') as f:
            f.write(f"Found {len(conns)} connections\n")
            
            for i, conn in enumerate(conns):
                if isinstance(conn, str):
                    print(f"Connection {i} is a string: {conn}", flush=True)
                    continue
                    
                cid = conn.get('id')
                cname = conn.get('name')
                ctype = conn.get('source_type', 'unknown')
                msg = f"\nChecking Connection: {cname} ({cid}) [{ctype}]"
                print(msg, flush=True)
                f.write(msg + "\n")
                
                # 2. Get tables for this connection
                t_resp = requests.get(f"http://localhost:3000/api/connections/{cid}/tables")
                tables = t_resp.json()
                if isinstance(tables, dict) and 'data' in tables:
                    tables = tables['data']
                
                msg = f"  Total tables: {len(tables)}"
                print(msg, flush=True)
                f.write(msg + "\n")
                
                # 3. Check for BMSQL/TPCC tables
                bmsql_count = 0
                print(f"  DEBUG: Raw Tables Response for {cname}:", flush=True)
                print(json.dumps(tables, indent=2, ensure_ascii=False), flush=True)
                
                for t in tables:
                    if isinstance(t, str):
                        t_name = t
                        schema = ''
                    else:
                        t_name = t.get('table_name', '')
                        schema = t.get('schema_name', '')
                    
                    if 'BMSQL' in t_name.upper() or 'TPCC' in t_name.upper():
                        msg = f"  Found target table: {schema}.{t_name}"
                        print(msg, flush=True)
                        f.write(msg + "\n")
                        bmsql_count += 1
                
                if bmsql_count == 0:
                    msg = "  No BMSQL/TPCC tables found."
                    print(msg, flush=True)
                    f.write(msg + "\n")
  
    except Exception as e:
        print(f"Error: {e}", flush=True)
        import traceback
        traceback.print_exc(file=sys.stdout)

if __name__ == "__main__":
    check_all()
