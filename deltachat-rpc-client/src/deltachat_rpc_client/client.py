"""Event loop implementations offering high level event handling/hooking."""
import inspect
import logging
from typing import (
    TYPE_CHECKING,
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

from ._utils import (
    AttrDict,
    parse_system_add_remove,
    parse_system_image_changed,
    parse_system_title_changed,
)
from .const import COMMAND_PREFIX, EventType, SpecialContactId, SystemMessageType
from .events import (
    EventFilter,
    GroupImageChanged,
    GroupNameChanged,
    MemberListChanged,
    NewMessage,
    RawEvent,
)

if TYPE_CHECKING:
    from deltachat_rpc_client.account import Account


class Client:
    """Simple Delta Chat client that listen to events of a single account."""

    def __init__(
        self,
        account: "Account",
        hooks: Optional[Iterable[Tuple[Callable, Union[type, EventFilter]]]] = None,
        logger: Optional[logging.Logger] = None,
    ) -> None:
        self.account = account
        self.logger = logger or logging
        self._hooks: Dict[type, Set[tuple]] = {}
        self._should_process_messages = 0
        self.add_hooks(hooks or [])

    def add_hooks(self, hooks: Iterable[Tuple[Callable, Union[type, EventFilter]]]) -> None:
        for hook, event in hooks:
            self.add_hook(hook, event)

    def add_hook(self, hook: Callable, event: Union[type, EventFilter] = RawEvent) -> None:
        """Register hook for the given event filter."""
        if isinstance(event, type):
            event = event()
        assert isinstance(event, EventFilter)
        self._should_process_messages += int(
            isinstance(
                event,
                (NewMessage, MemberListChanged, GroupImageChanged, GroupNameChanged),
            ),
        )
        self._hooks.setdefault(type(event), set()).add((hook, event))

    def remove_hook(self, hook: Callable, event: Union[type, EventFilter]) -> None:
        """Unregister hook from the given event filter."""
        if isinstance(event, type):
            event = event()
        self._should_process_messages -= int(
            isinstance(
                event,
                (NewMessage, MemberListChanged, GroupImageChanged, GroupNameChanged),
            ),
        )
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

    async def run_until(self, func: Callable[[AttrDict], Union[bool, Coroutine]]) -> AttrDict:
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

    async def _on_event(self, event: AttrDict, filter_type: Type[EventFilter] = RawEvent) -> None:
        for hook, evfilter in self._hooks.get(filter_type, []):
            if await evfilter.filter(event):
                try:
                    await hook(event)
                except Exception as ex:
                    self.logger.exception(ex)

    async def _parse_command(self, event: AttrDict) -> None:
        cmds = [hook[1].command for hook in self._hooks.get(NewMessage, []) if hook[1].command]
        parts = event.message_snapshot.text.split(maxsplit=1)
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

        event["command"], event["payload"] = cmd, payload

    async def _on_new_msg(self, snapshot: AttrDict) -> None:
        event = AttrDict(command="", payload="", message_snapshot=snapshot)
        if not snapshot.is_info and (snapshot.text or "").startswith(COMMAND_PREFIX):
            await self._parse_command(event)
        await self._on_event(event, NewMessage)

    async def _handle_info_msg(self, snapshot: AttrDict) -> None:
        event = AttrDict(message_snapshot=snapshot)

        img_changed = parse_system_image_changed(snapshot.text)
        if img_changed:
            _, event["image_deleted"] = img_changed
            await self._on_event(event, GroupImageChanged)
            return

        title_changed = parse_system_title_changed(snapshot.text)
        if title_changed:
            _, event["old_name"] = title_changed
            await self._on_event(event, GroupNameChanged)
            return

        members_changed = parse_system_add_remove(snapshot.text)
        if members_changed:
            action, event["member"], _ = members_changed
            event["member_added"] = action == "added"
            await self._on_event(event, MemberListChanged)
            return

        self.logger.warning(
            "ignoring unsupported system message id=%s text=%s",
            snapshot.id,
            snapshot.text,
        )

    async def _process_messages(self) -> None:
        if self._should_process_messages:
            for message in await self.account.get_next_messages():
                snapshot = await message.get_snapshot()
                if snapshot.from_id not in [SpecialContactId.SELF, SpecialContactId.DEVICE]:
                    await self._on_new_msg(snapshot)
                if snapshot.is_info and snapshot.system_message_type != SystemMessageType.WEBXDC_INFO_MESSAGE:
                    await self._handle_info_msg(snapshot)
                await snapshot.message.mark_seen()


class Bot(Client):
    """Simple bot implementation that listent to events of a single account."""

    async def configure(self, email: str, password: str, **kwargs) -> None:
        kwargs.setdefault("bot", "1")
        await super().configure(email, password, **kwargs)
