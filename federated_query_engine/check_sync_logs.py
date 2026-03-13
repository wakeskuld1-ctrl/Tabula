
import requests
import json
import time

def get_logs():
    url = "http://localhost:3000/api/logs"
    print("Checking logs for Syncing activity...")
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
        
        found = []
        for log in logs:
            if "Syncing YashanDB" in log or "Auto-discovering" in log:
                found.append(log)
        
        if found:
            print(f"Found {len(found)} sync logs:")
            for log in found[-20:]:
                print(log)
        else:
            print("No sync logs found yet.")
            
    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    get_logs()
