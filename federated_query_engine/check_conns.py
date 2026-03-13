
import requests
import json

def check_connections():
    url = "http://localhost:3000/api/connections"
    try:
        response = requests.get(url)
        print(f"Response: {response.text}")
        conns = response.json()
        if isinstance(conns, dict):
            if "connections" in conns:
                conns = conns["connections"]
            elif "data" in conns:
                conns = conns["data"]

        print(f"Found {len(conns)} connections")
        for c in conns:
            # print(c)
            if isinstance(c, dict):
                print(f"ID: {c.get('id')} Name: {c.get('name')} Type: {c.get('type')}")
            else:
                print(f"Unexpected item type: {type(c)} - {c}")
    except Exception as e:
        print(f"Exception: {e}")

if __name__ == "__main__":
    check_connections()
