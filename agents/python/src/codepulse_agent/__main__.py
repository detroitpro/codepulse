"""CLI entry for the Python agent."""

from __future__ import annotations

import argparse
import os
import runpy
import sys


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="codepulse-agent")
    sub = parser.add_subparsers(dest="cmd")

    run_p = sub.add_parser("run", help="Run a Python module/script under codepulse")
    run_p.add_argument("target", help="script path or -m module")
    run_p.add_argument("args", nargs=argparse.REMAINDER)

    sub.add_parser("install-info", help="Print env/session info after install")

    args = parser.parse_args(argv)

    if args.cmd == "install-info":
        from .agent import install

        sid = install()
        print(f"session_id={sid}")
        print(f"endpoint={os.environ.get('CODEPULSE_ENDPOINT', 'http://127.0.0.1:7420')}")
        return

    if args.cmd == "run":
        from .agent import install

        install()
        target_args = list(args.args)
        if target_args and target_args[0] == "--":
            target_args = target_args[1:]
        if args.target == "-m":
            if not target_args:
                parser.error("-m requires a module name")
            mod = target_args[0]
            sys.argv = [mod, *target_args[1:]]
            runpy.run_module(mod, run_name="__main__", alter_sys=True)
        else:
            sys.argv = [args.target, *target_args]
            runpy.run_path(args.target, run_name="__main__")
        return

    parser.print_help()


if __name__ == "__main__":
    main()
