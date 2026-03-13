import re

with open("yascli.dll", "rb") as f:
    content = f.read()
    strings = re.findall(b"yac[A-Za-z0-9_]+", content)
    for s in strings:
        print(s.decode('utf-8', errors='ignore'))
