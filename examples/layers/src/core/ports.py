"""What core needs from the outside, as interfaces app/ implements."""

from typing import Protocol


class Rates(Protocol):
    def for_region(self, region: str) -> float: ...
