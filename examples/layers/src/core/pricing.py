"""Order pricing."""

from core.ports import Rates


def price(order, rates: Rates):
    return rates.for_region(order.region) * order.total
