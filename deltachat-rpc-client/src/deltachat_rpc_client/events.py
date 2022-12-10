"""High-level classes for event processing and filtering."""
import inspect
import re
from abc import ABC, abstractmethod
from typing import Callable, Iterable, Iterator, Optional, Set, Tuple, Union

from .const import EventType
from .utils import AttrDict


def _tuple_of(obj, type_: type) -> tuple:
    if not obj:
        return tuple()
    if isinstance(obj, type_):
        obj = (obj,)

    if not all(isinstance(elem, type_) for elem in obj):
        raise TypeError()
    return tuple(obj)


class EventFilter(ABC):
    """The base event filter.

    :param func: A Callable (async or not) function that should accept the event as input
                 parameter, and return a bool value indicating whether the event
                 should be dispatched or not.
    """

    def __init__(self, func: Optional[Callable] = None):
        self.func = func

    @abstractmethod
    def __hash__(self) -> int:
        """Object's unique hash"""

    @abstractmethod
    def __eq__(self, other) -> bool:
        """Return True if two event filters are equal."""

    def __ne__(self, other):
        return not self.__eq__(other)

    async def _call_func(self, event) -> bool:
        if not self.func:
            return True
        res = self.func(event)
        if inspect.isawaitable(res):
            return await res
        return res

    @abstractmethod
    async def filter(self, event):
        """Return True-like value if the event passed the filter and should be
        used, or False-like value otherwise.
        """


class RawEvent(EventFilter):
    """Matches raw core events.

    :param types: The types of event to match.
    :param func: A Callable (async or not) function that should accept the event as input
                 parameter, and return a bool value indicating whether the event
                 should be dispatched or not.
    """

    def __init__(
        self, types: Union[None, EventType, Iterable[EventType]] = None, **kwargs
    ):
        super().__init__(**kwargs)
        try:
            self.types = _tuple_of(types, EventType)
        except TypeError as err:
            raise TypeError(f"Invalid event type given: {types}") from err

    def __hash__(self) -> int:
        return hash((self.types, self.func))

    def __eq__(self, other) -> bool:
        if isinstance(other, RawEvent):
            return (self.types, self.func) == (other.types, other.func)
        return False

    async def filter(self, event: AttrDict) -> bool:
        if self.types and event.type not in self.types:
            return False
        return await self._call_func(event)


class NewMessage(EventFilter):
    """Matches whenever a new message arrives.

    Warning: registering a handler for this event or any subclass will cause the messages
    to be marked as read. Its usage is mainly intended for bots.
    """

    def __init__(
        self,
        pattern: Union[
            None,
            str,
            Callable[[str], bool],
            re.Pattern,
        ] = None,
        func: Optional[Callable[[AttrDict], bool]] = None,
    ) -> None:
        super().__init__(func=func)
        if isinstance(pattern, str):
            pattern = re.compile(pattern)
        if isinstance(pattern, re.Pattern):
            self.pattern: Optional[Callable] = pattern.match
        elif not pattern or callable(pattern):
            self.pattern = pattern
        else:
            raise TypeError("Invalid pattern type")

    def __hash__(self) -> int:
        return hash((self.pattern, self.func))

    def __eq__(self, other) -> bool:
        if type(other) is self.__class__:  # noqa
            return (self.pattern, self.func) == (other.pattern, other.func)
        return False

    async def filter(self, event: AttrDict) -> bool:
        if self.pattern:
            match = self.pattern(event.text)
            if inspect.isawaitable(match):
                match = await match
            if not match:
                return False
        return await super()._call_func(event)


class NewInfoMessage(NewMessage):
    """Matches whenever a new info/system message arrives."""


class HookCollection:
    """
    Helper class to collect event hooks that can later be added to a Delta Chat client.
    """

    def __init__(self) -> None:
        self._hooks: Set[Tuple[Callable, Union[type, EventFilter]]] = set()

    def __iter__(self) -> Iterator[Tuple[Callable, Union[type, EventFilter]]]:
        return iter(self._hooks)

    def on(self, event: Union[type, EventFilter]) -> Callable:  # noqa
        """Register decorated function as listener for the given event."""
        if isinstance(event, type):
            event = event()
        assert isinstance(event, EventFilter), "Invalid event filter"

        def _decorator(func) -> Callable:
            self._hooks.add((func, event))
            return func

        return _decorator
