import argparse
import asyncio
import re
import sys
from typing import TYPE_CHECKING, Callable, Iterable, Optional, Tuple, Type, Union

if TYPE_CHECKING:
    from .client import Client
    from .events import EventFilter


def _camel_to_snake(name: str) -> str:
    name = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", name)
    name = re.sub("__([A-Z])", r"_\1", name)
    name = re.sub("([a-z0-9])([A-Z])", r"\1_\2", name)
    return name.lower()


def _to_attrdict(obj):
    if isinstance(obj, dict):
        return AttrDict(obj)
    if isinstance(obj, list):
        return [_to_attrdict(elem) for elem in obj]
    return obj


class AttrDict(dict):
    """Dictionary that allows accessing values usin the "dot notation" as attributes."""

    def __init__(self, *args, **kwargs) -> None:
        super().__init__(
            {
                _camel_to_snake(key): _to_attrdict(value)
                for key, value in dict(*args, **kwargs).items()
            }
        )

    def __getattr__(self, attr):
        if attr in self:
            return self[attr]
        raise AttributeError("Attribute not found: " + str(attr))

    def __setattr__(self, attr, val):
        if attr in self:
            raise AttributeError("Attribute-style access is read only")
        super().__setattr__(attr, val)


async def run_client_cli(
    hooks: Optional[Iterable[Tuple[Callable, Union[type, "EventFilter"]]]] = None,
    argv: Optional[list] = None,
    **kwargs
) -> None:
    """Run a simple command line app, using the given hooks.

    Extra keyword arguments are passed to the internal Rpc object.
    """
    from .client import Client

    await _run_cli(Client, hooks, argv, **kwargs)


async def run_bot_cli(
    hooks: Optional[Iterable[Tuple[Callable, Union[type, "EventFilter"]]]] = None,
    argv: Optional[list] = None,
    **kwargs
) -> None:
    """Run a simple bot command line using the given hooks.

    Extra keyword arguments are passed to the internal Rpc object.
    """
    from .client import Bot

    await _run_cli(Bot, hooks, argv, **kwargs)


async def _run_cli(
    client_type: Type["Client"],
    hooks: Optional[Iterable[Tuple[Callable, Union[type, "EventFilter"]]]] = None,
    argv: Optional[list] = None,
    **kwargs
) -> None:
    from .deltachat import DeltaChat
    from .rpc import Rpc

    if argv is None:
        argv = sys.argv

    parser = argparse.ArgumentParser(prog=argv[0] if argv else None)
    parser.add_argument(
        "accounts_dir",
        help="accounts folder (default: current working directory)",
        nargs="?",
    )
    parser.add_argument("--email", action="store", help="email address")
    parser.add_argument("--password", action="store", help="password")
    args = parser.parse_args(argv[1:])

    async with Rpc(accounts_dir=args.accounts_dir, **kwargs) as rpc:
        deltachat = DeltaChat(rpc)
        core_version = (await deltachat.get_system_info()).deltachat_core_version
        accounts = await deltachat.get_all_accounts()
        account = accounts[0] if accounts else await deltachat.add_account()

        client = client_type(account, hooks)
        client.logger.debug("Running deltachat core %s", core_version)
        if not await client.is_configured():
            assert (
                args.email and args.password
            ), "Account is not configured and email and password must be provided"
            asyncio.create_task(
                client.configure(email=args.email, password=args.password)
            )
        await client.run_forever()
