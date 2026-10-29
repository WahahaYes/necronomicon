# /// script
# requires-python = ">=3.9"
# ///
"""The suite's canned command: prints canned.txt and canned-stderr.txt from its
working folder, then exits with T_EXIT."""

import os
import sys
from pathlib import Path

for name, stream in (("canned.txt", sys.stdout), ("canned-stderr.txt", sys.stderr)):
    if Path(name).exists():
        stream.write(Path(name).read_text(encoding="utf-8"))
sys.exit(int(os.environ.get("T_EXIT", "0")))
