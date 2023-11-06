import concurrent.futures
import logging

from deltachat_rpc_client.rpc import Rpc


def main() -> None:
    """Run lots of parallel calls to stress-test threading and synchronization."""
    logging.basicConfig(encoding="utf-8", level=logging.INFO)
    with Rpc(accounts_dir="accounts") as rpc, concurrent.futures.ThreadPoolExecutor(max_workers=20) as executor:
        done, pending = concurrent.futures.wait(
            [executor.submit(rpc.sleep, 0.1) for i in range(1000)],
            return_when=concurrent.futures.ALL_COMPLETED,
        )


main()
