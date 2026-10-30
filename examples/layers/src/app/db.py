"""Database access; core reaches it only through core.ports."""

from core import pricing


class DbRates:
    def for_region(self, region: str) -> float:
        return {"eu": 1.2, "us": 1.0}.get(region, 1.0)


def price_order(order) -> float:
    return pricing.price(order, DbRates())
