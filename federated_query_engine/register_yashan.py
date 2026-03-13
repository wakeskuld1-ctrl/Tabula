import requests
import json
import time

def register():
    url = "http://localhost:3000/api/connections"
    payload = {
        "id": "conn_1770111588950",
        "name": "tpcc@192.168.23.4:1843/yashandb",
        "source_type": "yashandb",
        "config": json.dumps({
            "user": "tpcc",
            "pass": "tpcc",
            "host": "192.168.23.4",
            "port": 1843,
            "service": "yashandb"
        })
    }
    
    print(f"Registering connection: {payload}")
    try:
        resp = requests.post(url, json=payload)
        print(f"Status: {resp.status_code}")
        print(f"Response: {resp.text}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    # Wait for server to start
    for i in range(10):
        try:
            requests.get("http://localhost:3001/health")
            break
        except:
            print("Waiting for server...")
            time.sleep(2)
            
    register()
