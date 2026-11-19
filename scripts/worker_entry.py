"""PyInstaller entry point; all diagnostics remain on stderr."""
import sys
from flowforge.worker import serve

sys.stdin.reconfigure(encoding="utf-8")
sys.stdout.reconfigure(encoding="utf-8")
serve(sys.stdin, sys.stdout)
