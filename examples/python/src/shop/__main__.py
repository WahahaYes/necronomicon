"""The command line: python -m shop 1.5 2.25 prints the total."""

import sys

from shop.cart import total

print(total([float(a) for a in sys.argv[1:]]))  # necro-allow no-print: the CLI's output is stdout
