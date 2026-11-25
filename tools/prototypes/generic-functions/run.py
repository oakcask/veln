"""Run the bounded proposal experiment with Python 3.10+ and JDK 17+."""

from pathlib import Path
import subprocess
import tempfile
import unittest

from surface import emit_java, parse_program


def main():
    root = Path(__file__).resolve().parent
    suite = unittest.defaultTestLoader.discover(str(root), pattern="test_*.py")
    outcome = unittest.TextTestRunner(verbosity=2).run(suite)
    if not outcome.wasSuccessful():
        raise SystemExit(1)
    source = (root / "functions.veln").read_text()
    with tempfile.TemporaryDirectory(prefix="veln-generic-prototype-") as temporary:
        build = Path(temporary)
        provider = build / "Provider.java"
        provider.write_text(emit_java(parse_program(source)))
        # Compile the consumer separately: it only sees the provider class files.
        subprocess.run(["javac", "--release", "17", "-d", temporary, str(provider)], check=True, timeout=30)
        subprocess.run(["javac", "--release", "17", "-cp", temporary, "-d", temporary,
                        str(root / "Consumer.java")], check=True, timeout=30)
        subprocess.run(["java", "-Xmx128m", "-Xverify:all", "-cp", temporary, "Consumer"], check=True, timeout=30)


if __name__ == "__main__":
    main()
