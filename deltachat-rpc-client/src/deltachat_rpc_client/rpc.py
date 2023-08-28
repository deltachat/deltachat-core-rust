import asyncio
import json
import os
from typing import Any, Dict, Optional


class JsonRpcError(Exception):
    pass


class Rpc:
    def __init__(self, accounts_dir: Optional[str] = None, **kwargs):
        """The given arguments will be passed to asyncio.create_subprocess_exec()"""
        if accounts_dir:
            kwargs["env"] = {
                **kwargs.get("env", os.environ),
                "DC_ACCOUNTS_PATH": str(accounts_dir),
            }

        self._kwargs = kwargs
        self.process: asyncio.subprocess.Process
        self.id: int
        self.event_queues: Dict[int, asyncio.Queue]
        # Map from request ID to `asyncio.Future` returning the response.
        self.request_events: Dict[int, asyncio.Future]
        self.closing: bool
        self.reader_task: asyncio.Task
        self.events_task: asyncio.Task

    async def start(self) -> None:
        self.process = await asyncio.create_subprocess_exec(
            "deltachat-rpc-server",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            **self._kwargs,
        )
        self.id = 0
        self.event_queues = {}
        self.request_events = {}
        self.closing = False
        self.reader_task = asyncio.create_task(self.reader_loop())
        self.events_task = asyncio.create_task(self.events_loop())

    async def close(self) -> None:
        """Terminate RPC server process and wait until the reader loop finishes."""
        self.closing = True
        await self.stop_io_for_all_accounts()
        await self.events_task
        self.process.terminate()
        await self.reader_task

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, _exc_type, _exc, _tb):
        await self.close()

    async def reader_loop(self) -> None:
        while True:
            line = await self.process.stdout.readline()  # noqa
            if not line:  # EOF
                break
            response = json.loads(line)
            if "id" in response:
                fut = self.request_events.pop(response["id"])
                fut.set_result(response)
            else:
                print(response)

    async def get_queue(self, account_id: int) -> asyncio.Queue:
        if account_id not in self.event_queues:
            self.event_queues[account_id] = asyncio.Queue()
        return self.event_queues[account_id]

    async def events_loop(self) -> None:
        """Requests new events and distributes them between queues."""
        while True:
            if self.closing:
                return
            event = await self.get_next_event()
            account_id = event["contextId"]
            queue = await self.get_queue(account_id)
            await queue.put(event["event"])

    async def wait_for_event(self, account_id: int) -> Optional[dict]:
        """Waits for the next event from the given account and returns it."""
        queue = await self.get_queue(account_id)
        return await queue.get()

    def __getattr__(self, attr: str):
        async def method(*args) -> Any:
            self.id += 1
            request_id = self.id

            request = {
                "jsonrpc": "2.0",
                "method": attr,
                "params": args,
                "id": self.id,
            }
            data = (json.dumps(request) + "\n").encode()
            self.process.stdin.write(data)  # noqa
            loop = asyncio.get_running_loop()
            fut = loop.create_future()
            self.request_events[request_id] = fut
            response = await fut
            if "error" in response:
                raise JsonRpcError(response["error"])
            if "result" in response:
                return response["result"]
            return None

        return method
