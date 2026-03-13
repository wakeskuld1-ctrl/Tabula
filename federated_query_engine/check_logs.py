
import requests
import json

def get_logs():
    url = "http://localhost:3000/api/logs"
    try:
        response = requests.get(url)
        data = response.json()
        
        logs = []
        if isinstance(data, list):
            logs = data
        elif isinstance(data, dict):
            if "logs" in data:
                logs = data["logs"]
        
        print(f"Total log entries: {len(logs)}")
        
        # Search for TPCC or BMSQL
        found = []
        for log in logs:
            if "TPCC" in log.upper() or "BMSQL" in log.upper():
                found.append(log)
        
        if found:
            print(f"Found {len(found)} TPCC/BMSQL related logs:")
            for log in found[-20:]: # Last 20
                print(log)
        else:
            print("No TPCC/BMSQL logs found.")
            
    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    get_logs()
