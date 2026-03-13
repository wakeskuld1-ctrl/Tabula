import requests
import json

def simulate_normalizer(ident):
    """
    Simulates the logic found in query_rewriter.rs IdentifierNormalizer::parse
    """
    parts = ident.split('.')
    if len(parts) == 3:
        # catalog.schema.table -> (Some(schema), table)
        schema = parts[1].strip('"').strip("'")
        table = parts[2].strip('"').strip("'")
        return schema, table
    elif len(parts) == 2:
        # schema.table -> (Some(schema), table)
        schema = parts[0].strip('"').strip("'")
        table = parts[1].strip('"').strip("'")
        return schema, table
    else:
        # table -> (None, table)
        table = ident.strip('"').strip("'")
        return None, table

def check_registered_tables():
    """
    Fetches registered tables and checks if the normalized user query matches any.
    """
    try:
        response = requests.get("http://localhost:3000/api/tables")
        if response.status_code == 200:
            data = response.json()
            # Handle different response structures based on previous findings
            if isinstance(data, dict) and "tables" in data:
                registered_tables = data["tables"]
            elif isinstance(data, list):
                registered_tables = data
            else:
                print(f"Unknown response format: {type(data)}")
                return

            print(f"Found {len(registered_tables)} registered tables.")
            
            # User query example: "tpcc.bmsql_warehouse"
            user_query_table = "tpcc.bmsql_warehouse"
            norm_schema, norm_table = simulate_normalizer(user_query_table)
            print(f"\nUser Query: {user_query_table}")
            print(f"Normalized: Schema='{norm_schema}', Table='{norm_table}'")
            
            print("\nChecking against registered tables:")
            found = False
            for reg_table in registered_tables:
                # Registered table format usually: catalog.schema.table or just table
                # Based on previous check, they look like: TPCC.oracle_tpcc_b5f81ede29f6cad1_bmsql_warehouse
                print(f"  - Registered: {reg_table}")
                
                # Check for direct match
                if reg_table == user_query_table:
                    print("    -> DIRECT MATCH FOUND")
                    found = True
                    continue
                
                # Check if the registered table ends with the normalized table name
                if reg_table.endswith(norm_table):
                    print(f"    -> PARTIAL MATCH (Suffix): {reg_table} ends with {norm_table}")
                    # But the prefix might be different
                    if norm_schema and norm_schema.lower() in reg_table.lower():
                         print("    -> SCHEMA MATCH indicates this might be the target")
                    else:
                         print("    -> SCHEMA MISMATCH")
                
            if not found:
                print("\nRESULT: No direct match found. The rewriter logic likely fails to map 'tpcc.bmsql_warehouse' to the hashed internal name.")
                
        else:
            print(f"Failed to fetch tables: {response.status_code}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    check_registered_tables()
