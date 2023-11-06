import concurrent.futures
import json
import logging
import os
import subprocess
import sys
from queue import Queue
from threading import Event, Thread, Lock
from typing import Any, Dict, Optional


class JsonRpcError(Exception):
    pass


class Rpc:
    def __init__(self, accounts_dir: Optional[str] = None, **kwargs):
        """The given arguments will be passed to subprocess.Popen()"""
        if accounts_dir:
            kwargs["env"] = {
                **kwargs.get("env", os.environ),
                "DC_ACCOUNTS_PATH": str(accounts_dir),
            }

        self._kwargs = kwargs
        self.process: subprocess.Popen
        self.id: int
        self.id_lock: Lock
        # Map from request ID to `threading.Event`.
        self.request_events: Dict[int, Event]
        # Map from request ID to the result.
        self.request_results: Dict[int, Any]
        self.request_queue: Queue[Any]
        self.closing: bool
        self.reader_thread: Thread
        self.writer_thread: Thread

    def start(self) -> None:
        if sys.version_info >= (3, 11):
            self.process = subprocess.Popen(
                "deltachat-rpc-server",
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                # Prevent subprocess from capturing SIGINT.
                process_group=0,
                **self._kwargs,
            )
        else:
            self.process = subprocess.Popen(
                "deltachat-rpc-server",
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                # `process_group` is not supported before Python 3.11.
                preexec_fn=os.setpgrp,  # noqa: PLW1509
                **self._kwargs,
            )
        self.id = 0
        self.id_lock = Lock()
        self.request_events = {}
        self.request_results = {}
        self.request_queue = Queue()
        self.closing = False
        self.reader_thread = Thread(target=self.reader_loop)
        self.reader_thread.start()
        self.writer_thread = Thread(target=self.writer_loop)
        self.writer_thread.start()

    def close(self) -> None:
        """Terminate RPC server process and wait until the reader loop finishes."""
        self.closing = True
        self.process.stdin.close()
        self.reader_thread.join()
        self.request_queue.put(None)
        self.writer_thread.join()

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, _exc_type, _exc, _tb):
        self.close()

    def reader_loop(self) -> None:
        try:
            while True:
                line = self.process.stdout.readline()
                if not line:  # EOF
                    break
                response = json.loads(line)
                if "id" in response:
                    response_id = response["id"]
                    event = self.request_events.pop(response_id)
                    self.request_results[response_id] = response
                    event.set()
                else:
                    logging.warning("Got a response without ID: %s", response)
        except Exception:
            # Log an exception if the reader loop dies.
            logging.exception("Exception in the reader loop")
            raise

    def writer_loop(self) -> None:
        """Writer loop ensuring only a single thread writes requests."""
        try:
            while True:
                request = self.request_queue.get()
                if not request:
                    break
                data = (json.dumps(request) + "\n").encode()
                self.process.stdin.write(data)
                self.process.stdin.flush()

        except Exception:
            # Log an exception if the writer loop dies.
            logging.exception("Exception in the writer loop")
            raise

    def __getattr__(self, attr: str):
        def method(*args) -> Any:
            self.id_lock.acquire()
            self.id += 1
            request_id = self.id
            self.id_lock.release()

            request = {
                "jsonrpc": "2.0",
                "method": attr,
                "params": args,
                "id": self.id,
            }
            event = Event()
            self.request_events[request_id] = event
            self.request_queue.put(request)
            event.wait()

            response = self.request_results.pop(request_id)
            if "error" in response:
                raise JsonRpcError(response["error"])
            if "result" in response:
                return response["result"]
            return None

        return method


def main() -> None:
    """Run lots of parallel calls to stress-test threading and synchronization."""
    logging.basicConfig(encoding="utf-8", level=logging.INFO)
    with Rpc(accounts_dir="accounts") as rpc, concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        done, pending = concurrent.futures.wait(
            (executor.submit(rpc.sleep, 0.0) for i in range(10000)),
            return_when=concurrent.futures.ALL_COMPLETED,
        )

if __name__ == "__main__":
    main()
