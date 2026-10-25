#!/usr/bin/env python3
"""Replays a scenario in a live editor, one instruction at a time (ADR 26, 27).

In one terminal:   cargo run -- --socket /tmp/ned.sock --sandbox
In another:        tools/replay.py tests/scenarios/search/enter-starts-and-f3-walks-forward-and-back.sc

Every instruction is shown and sent on Enter. `a` sends the rest without
stopping, `q` stops. At a snapshot the editor answers with the frame it
shows, which is printed beside the frame in the file, with the lines that
differ marked.
"""

import socket
import sys

SOCKET = "/tmp/ned.sock"


def read_scenario(path):
    """The author's lines and, under each snapshot, the frame the file has.

    Yields (line, frame): frame is None for an instruction, a list of lines
    for a snapshot. Frames are what follows `---` up to a blank line.
    """
    with open(path, encoding="utf-8") as file:
        lines = [line.rstrip("\n") for line in file]
    index = 0
    while index < len(lines):
        line = lines[index]
        index += 1
        if line.startswith("---"):
            frame = []
            while index < len(lines) and lines[index] != "":
                frame.append(lines[index])
                index += 1
            yield line, frame
        elif line.strip() and not line.startswith("#"):
            yield line, None


def connect(path):
    connection = socket.socket(socket.AF_UNIX)
    connection.connect(path)
    return connection.makefile("rw", encoding="utf-8")


def ask(stream, line):
    """Sends one line and reads the answer: lines up to a blank one."""
    stream.write(line + "\n")
    stream.flush()
    answer = []
    while True:
        received = stream.readline()
        if received in ("", "\n"):
            return answer
        answer.append(received.rstrip("\n"))


def show(expected, shown):
    """The two frames side by side; a line that differs is marked with `!`."""
    width = max([len(line) for line in expected] + [10])
    print(f"    {'file':<{width}}   editor")
    for index in range(max(len(expected), len(shown))):
        want = expected[index] if index < len(expected) else ""
        got = shown[index] if index < len(shown) else ""
        mark = " " if want == got else "!"
        print(f"  {mark} {want:<{width}}   {got}")


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    path = sys.argv[1]
    stream = connect(sys.argv[2] if len(sys.argv) > 2 else SOCKET)

    running = False
    mismatches = 0
    for line, frame in read_scenario(path):
        if not running:
            try:
                command = input(f"{line}  ")
            except EOFError:
                return
            if command == "q":
                return
            if command == "a":
                running = True
        else:
            print(line)

        answer = ask(stream, line)
        if frame is None:
            if answer != ["ok"]:
                print("   ", *answer)
            continue
        if answer != frame:
            mismatches += 1
        show(frame, answer)
    print(f"done, {mismatches} snapshot(s) differ" if mismatches else "done, every snapshot agrees")


if __name__ == "__main__":
    main()
