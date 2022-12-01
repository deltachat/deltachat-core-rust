import asyncio
import json
from typing import Any, Dict, Optional


class JsonRpcError(Exception):
    pass


class Rpc:
    def __init__(self, process: asyncio.subprocess.Process) -> None:
        self.process = process
        self.event_queues: Dict[int, asyncio.Queue] = {}
        self.id = 0
        self.reader_task = asyncio.create_task(self.reader_loop())

        # Map from request ID to `asyncio.Future` returning the response.
        self.request_events: Dict[int, asyncio.Future] = {}

    async def reader_loop(self) -> None:
        while True:
            line = await self.process.stdout.readline()
            response = json.loads(line)
            if "id" in response:
                fut = self.request_events.pop(response["id"])
                fut.set_result(response)
            elif response["method"] == "event":
                # An event notification.
                params = response["params"]
                account_id = params["contextId"]
                if account_id not in self.event_queues:
                    self.event_queues[account_id] = asyncio.Queue()
                await self.event_queues[account_id].put(params["event"])
            else:
                print(response)

    async def wait_for_event(self, account_id: int) -> Optional[dict]:
        """Waits for the next event from the given account and returns it."""
        if account_id in self.event_queues:
            return await self.event_queues[account_id].get()
        return None

    def __getattr__(self, attr: str):
        async def method(*args, **kwargs) -> Any:
            self.id += 1
            request_id = self.id

            assert not (args and kwargs), "Mixing positional and keyword arguments"

            request = {
                "jsonrpc": "2.0",
                "method": attr,
                "params": kwargs or args,
                "id": self.id,
            }
            data = (json.dumps(request) + "\n").encode()
            self.process.stdin.write(data)
            loop = asyncio.get_running_loop()
            fut = loop.create_future()
            self.request_events[request_id] = fut
            response = await fut
            if "error" in response:
                raise JsonRpcError(response["error"])
            if "result" in response:
                return response["result"]

        return method


async def start_rpc_server(*args, **kwargs) -> Rpc:
    proc = await asyncio.create_subprocess_exec(
        "deltachat-rpc-server",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        *args,
        **kwargs
    )
    rpc = Rpc(proc)
    return rpc
