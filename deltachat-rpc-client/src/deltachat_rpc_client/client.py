"""Event loop implementations offering high level event handling/hooking."""
import inspect
import logging
from typing import (
    Callable,
    Coroutine,
    Dict,
    Iterable,
    Optional,
    Set,
    Tuple,
    Type,
    Union,
)

from deltachat_rpc_client.account import Account

from .const import COMMAND_PREFIX, EventType
from .events import EventFilter, NewMessage, RawEvent
from .utils import AttrDict


class Client:
    """Simple Delta Chat client that listen to events of a single account."""

    def __init__(
        self,
        account: Account,
        hooks: Optional[Iterable[Tuple[Callable, Union[type, EventFilter]]]] = None,
        logger: Optional[logging.Logger] = None,
    ) -> None:
        self.account = account
        self.logger = logger or logging
        self._hooks: Dict[type, Set[tuple]] = {}
        self.add_hooks(hooks or [])

    def add_hooks(
        self, hooks: Iterable[Tuple[Callable, Union[type, EventFilter]]]
    ) -> None:
        for hook, event in hooks:
            self.add_hook(hook, event)

    def add_hook(
        self, hook: Callable, event: Union[type, EventFilter] = RawEvent
    ) -> None:
        """Register hook for the given event filter."""
        if isinstance(event, type):
            event = event()
        assert isinstance(event, EventFilter)
        self._hooks.setdefault(type(event), set()).add((hook, event))

    def remove_hook(self, hook: Callable, event: Union[type, EventFilter]) -> None:
        """Unregister hook from the given event filter."""
        if isinstance(event, type):
            event = event()
        self._hooks.get(type(event), set()).remove((hook, event))

    async def is_configured(self) -> bool:
        return await self.account.is_configured()

    async def configure(self, email: str, password: str, **kwargs) -> None:
        await self.account.set_config("addr", email)
        await self.account.set_config("mail_pw", password)
        for key, value in kwargs.items():
            await self.account.set_config(key, value)
        await self.account.configure()
        self.logger.debug("Account configured")

    async def run_forever(self) -> None:
        """Process events forever."""
        await self.run_until(lambda _: False)

    async def run_until(
        self, func: Callable[[AttrDict], Union[bool, Coroutine]]
    ) -> AttrDict:
        """Process events until the given callable evaluates to True.

        The callable should accept an AttrDict object representing the
        last processed event. The event is returned when the callable
        evaluates to True.
        """
        self.logger.debug("Listening to incoming events...")
        if await self.is_configured():
            await self.account.start_io()
        await self._process_messages()  # Process old messages.
        while True:
            event = await self.account.wait_for_event()
            event["type"] = EventType(event.type)
            event["account"] = self.account
            await self._on_event(event)
            if event.type == EventType.INCOMING_MSG:
                await self._process_messages()

            stop = func(event)
            if inspect.isawaitable(stop):
                stop = await stop
            if stop:
                return event

    async def _on_event(
        self, event: AttrDict, filter_type: Type[EventFilter] = RawEvent
    ) -> None:
        for hook, evfilter in self._hooks.get(filter_type, []):
            if await evfilter.filter(event):
                try:
                    await hook(event)
                except Exception as ex:
                    self.logger.exception(ex)

    def _should_process_messages(self) -> bool:
        return NewMessage in self._hooks

    async def _parse_command(self, snapshot: AttrDict) -> None:
        cmds = [
            hook[1].command
            for hook in self._hooks.get(NewMessage, [])
            if hook[1].command
        ]
        parts = snapshot.text.split(maxsplit=1)
        payload = parts[1] if len(parts) > 1 else ""
        cmd = parts.pop(0)

        if "@" in cmd:
            suffix = "@" + (await self.account.self_contact.get_snapshot()).address
            if cmd.endswith(suffix):
                cmd = cmd[: -len(suffix)]
            else:
                return

        parts = cmd.split("_")
        _payload = payload
        while parts:
            _cmd = "_".join(parts)
            if _cmd in cmds:
                break
            _payload = (parts.pop() + " " + _payload).rstrip()

        if parts:
            cmd = _cmd
            payload = _payload

        snapshot["command"] = cmd
        snapshot["payload"] = payload

    async def _process_messages(self) -> None:
        if self._should_process_messages():
            for message in await self.account.get_fresh_messages_in_arrival_order():
                snapshot = await message.get_snapshot()
                snapshot["command"], snapshot["payload"] = "", ""
                if not snapshot.is_info and snapshot.text.startswith(COMMAND_PREFIX):
                    await self._parse_command(snapshot)
                await self._on_event(snapshot, NewMessage)
                await snapshot.message.mark_seen()


class Bot(Client):
    """Simple bot implementation that listent to events of a single account."""

    async def configure(self, email: str, password: str, **kwargs) -> None:
        kwargs.setdefault("bot", "1")
        await super().configure(email, password, **kwargs)
