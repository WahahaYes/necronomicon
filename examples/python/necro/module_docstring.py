# /// script
# requires-python = ">=3.11"
# ///
"""Print each tracked Python module (git pathspecs in argv) that has no docstring."""

import ast
import subprocess
import sys

files = subprocess.run(["git", "ls-files", "--", *sys.argv[1:]], capture_output=True, text=True, check=True)
for path in files.stdout.split():
    with open(path, encoding="utf-8") as f:
        if ast.get_docstring(ast.parse(f.read())) is None:
            print(path)
