import asyncio
import websockets
import json

connected = set()

async def handler(websocket):
    print("Client connected")
    connected.add(websocket)
    try:
        async for message in websocket:
            # Broadcast to all others
            for ws in connected:
                if ws != websocket:
                    await ws.send(message)
    except websockets.exceptions.ConnectionClosed:
        pass
    finally:
        connected.remove(websocket)
        print("Client disconnected")

async def main():
    async with websockets.serve(handler, "localhost", 8080):
        print("Server started on ws://localhost:8080")
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    asyncio.run(main())
