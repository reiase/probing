import os
import json

import asyncio
from websockets.asyncio.client import connect, unix_connect

async def client():
    endpoint = os.environ.get("PROBING_ENDPOINT", None)
    
    if "." in endpoint or ":" in endpoint:
        endpoint = connect(f"ws://{endpoint}/ws")
    else:
        endpoint = unix_connect(f"\0probing-{endpoint}", "ws://localhost/ws")
        
    async with endpoint as ws:
        while True:
            try:
                code = input(">>> ")
            except EOFError:
                print("\nExiting...")
                break

            await ws.send(f"{code}\n")

            rsp = await ws.recv()
            try:
                rsp = json.loads(rsp)
            except:
                rsp = {}
            if "output" in rsp:
                print(rsp["output"])
            if "traceback" in rsp and rsp["traceback"]:
                for line in rsp["traceback"]:
                    print(line)

def main():
    asyncio.run(client())