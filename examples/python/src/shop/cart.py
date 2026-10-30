"""Carts: items and their total."""

import logging

log = logging.getLogger(__name__)


def total(prices: list[float]) -> float:
    # print(prices) was the old debugging; comments don't count.
    log.debug("totalling %d prices", len(prices))
    return sum(prices)
