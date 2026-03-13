
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
        for log in logs:
            print(log)
            
    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    get_logs()
