#!/usr/bin/env python3
import sys
import threading
import time

PROMPT_PREFIX = "daedalus"
COMMANDS = {
    "help": "help",
    "desktop": "desk",
    "repeat": "r -",
    "pwd": "here",
    "ls": "list",
    "cd": "go",
    "cat": "read",
    "touch": "new",
    "mkdir": "make",
    "rm": "del",
    "echo": "say",
    "write": "write",
    "grep": "find",
    "nc": "ping",
    "sl": "train",
    "shw": "scan",
    "rmv": "remove",
    "run": "run",
    "start": "start",
    "stop": "stop",
    "reset": "reset",
    "ports": "ports",
    "peek": "peek",
    "sync": "sync",
}

TRAIN_ART = """      ====        ________                ___________
  _D _|  |_______/        \\__I_I_____===__|_________|
   |(_)---  |   H\\________/ |   |        =|___ ___|
   /     |  |   H  |  |     |   |         ||_| |_||
  |      |  |   H  |__--------------------| [___] |
  | ________|___H__/__|_____/[][]~\\_______|       |
  |/ |   |-----------I_____I [][] []  D   |=======|__
 __/ =| o |=-~~\\  /~~\\  /~~\\  /~~\\ ____Y___________|__
 |/-=|___|=O=====O=====O=====O=====|_____/~\\___/
"""


def create_file_system():
    return {
        "type": "dir",
        "children": {
            "home": {
                "type": "dir",
                "children": {
                    "daedalus": {
                        "type": "dir",
                        "children": {
                            "readme.txt": {
                                "type": "file",
                                "content": "Daedalus OS: simulated shell environment.\nType 'help' to explore commands.\n",
                            },
                            "notes.log": {
                                "type": "file",
                                "content": "loop: r - sys info\ninterrupt: ctrl+c\n",
                            },
                        },
                    }
                },
            },
            "sys": {
                "type": "dir",
                "children": {
                    "cpu.info": {"type": "file", "content": "RP2040 @133MHz\n"}
                },
            },
            "apps": {"type": "dir", "children": {}},
            "ports": {
                "type": "dir",
                "children": {"uart0.dev": {"type": "file", "content": "mock port\n"}},
            },
        },
    }


def normalize_path(path: str) -> str:
    parts = [p for p in path.split("/") if p]
    stack = []
    for part in parts:
        if part == ".":
            continue
        if part == "..":
            if stack:
                stack.pop()
            continue
        stack.append(part)
    return "/" + "/".join(stack)


def resolve_path(cwd: str, input_path: str) -> str:
    if not input_path or input_path == ".":
        return cwd
    if input_path.startswith("/"):
        return normalize_path(input_path)
    return normalize_path(f"{cwd}/{input_path}")


def get_node(fs_root, path: str):
    parts = [p for p in path.split("/") if p]
    current = fs_root
    for part in parts:
        if not current or current["type"] != "dir":
            return None
        current = current["children"].get(part)
    return current


def ensure_parent(fs_root, path: str):
    parts = [p for p in path.split("/") if p]
    if not parts:
        return None
    name = parts.pop()
    current = fs_root
    for part in parts:
        next_node = current["children"].get(part)
        if not next_node or next_node["type"] != "dir":
            return None
        current = next_node
    return current, name


def list_dir(node):
    if not node or node["type"] != "dir":
        return []
    return sorted(node["children"].keys())


class DaedalusCLI:
    def __init__(self):
        self.fs = create_file_system()
        self.cwd = "/home/daedalus"
        self.loops = []
        self.interrupt_token = 0

    def print_line(self, text=""):
        sys.stdout.write(text + "\n")
        sys.stdout.flush()

    def prompt(self):
        home_root = "/home/daedalus"
        suffix = self.cwd[len(home_root) :] if self.cwd.startswith(home_root) else self.cwd
        return f"{PROMPT_PREFIX}{suffix}:: "

    def write_prompt(self):
        sys.stdout.write(self.prompt())
        sys.stdout.flush()

    def start_repeat_loop(self, command: str):
        cancelled = threading.Event()
        token = self.interrupt_token

        def run_loop():
            while not cancelled.is_set() and token == self.interrupt_token:
                self.execute_command(command, from_loop=True)
                time.sleep(0.01)

        thread = threading.Thread(target=run_loop, daemon=True)
        thread.start()
        self.loops.append(cancelled)

    def handle_interrupt(self):
        self.interrupt_token += 1
        for loop in self.loops:
            loop.set()
        self.loops = []
        self.print_line("^C")

    def execute_command(self, input_line: str, from_loop: bool = False):
        if not input_line:
            if not from_loop:
                self.write_prompt()
            return

        if input_line == COMMANDS["help"]:
            self.print_line(
                "\n".join(
                    [
                        "help: show command list",
                        "desk: open desktop view",
                        "r - [cmd]: repeat a command until Ctrl+C",
                        "sys cpu|memory|info: system stats",
                        "here: print current path",
                        "list [path]: list directory",
                        "go [path]: change directory",
                        "read <file>: print file",
                        "new <file>: create empty file",
                        "make <dir>: create directory",
                        "del [-r] <path>: delete file or directory",
                        "say <text> [> file|>> file]: print or write",
                        "mod <file>: open editor (use .save/.cancel)",
                        "emod <file> <text>: overwrite file",
                        "write <file> <text>: overwrite file",
                        "find <pattern> <file>: search file",
                        "ping <host> <port>: mock connect",
                        "train: show train",
                        "scan/remove/run/start/stop/reset/ports/peek/sync: mock commands",
                    ]
                )
            )
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(COMMANDS["repeat"]):
            loop_command = input_line.replace(COMMANDS["repeat"], "", 1).strip()
            if not loop_command:
                self.print_line("Usage: r - [command]")
            else:
                self.start_repeat_loop(loop_command)
                self.print_line(f"Repeating: {loop_command}")
            if not from_loop:
                self.write_prompt()
            return

        if input_line == COMMANDS["desktop"]:
            self.print_line("Desktop mode is only available in the web UI.")
            if not from_loop:
                self.write_prompt()
            return

        if input_line == COMMANDS["pwd"]:
            self.print_line(self.cwd)
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(COMMANDS["ls"]):
            parts = input_line.split(" ")
            path_arg = parts[1] if len(parts) > 1 else "."
            target = resolve_path(self.cwd, path_arg)
            node = get_node(self.fs, target)
            if not node:
                self.print_line(f"list: cannot access '{path_arg}': No such file or directory")
            elif node["type"] == "file":
                self.print_line(target.split("/")[-1])
            else:
                self.print_line("  ".join(list_dir(node)))
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(COMMANDS["cd"]):
            parts = input_line.split(" ")
            path_arg = parts[1] if len(parts) > 1 else "/home/daedalus"
            target = resolve_path(self.cwd, path_arg)
            node = get_node(self.fs, target)
            if not node or node["type"] != "dir":
                self.print_line(f"go: {path_arg}: No such directory")
            else:
                self.cwd = target
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['cat']} "):
            _, path_arg = input_line.split(" ", 1)
            target = resolve_path(self.cwd, path_arg)
            node = get_node(self.fs, target)
            if not node or node["type"] != "file":
                self.print_line(f"read: {path_arg}: No such file")
            else:
                sys.stdout.write(node["content"])
                sys.stdout.flush()
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['touch']} "):
            _, path_arg = input_line.split(" ", 1)
            target = resolve_path(self.cwd, path_arg)
            entry = ensure_parent(self.fs, target)
            if not entry:
                self.print_line(f"new: cannot create '{path_arg}': No such directory")
            else:
                parent, name = entry
                parent["children"][name] = {"type": "file", "content": ""}
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['mkdir']} "):
            _, path_arg = input_line.split(" ", 1)
            target = resolve_path(self.cwd, path_arg)
            entry = ensure_parent(self.fs, target)
            if not entry:
                self.print_line(f"make: cannot create '{path_arg}': No such directory")
            else:
                parent, name = entry
                parent["children"][name] = {"type": "dir", "children": {}}
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['rm']} "):
            parts = input_line.split(" ")
            recursive = parts[1] in ("-r", "-rf") if len(parts) > 1 else False
            path_arg = parts[2] if recursive and len(parts) > 2 else (parts[1] if len(parts) > 1 else "")
            target = resolve_path(self.cwd, path_arg)
            entry = ensure_parent(self.fs, target)
            if not entry:
                self.print_line(f"del: cannot remove '{path_arg}': No such file or directory")
            else:
                parent, name = entry
                node = parent["children"].get(name)
                if not node:
                    self.print_line(f"del: cannot remove '{path_arg}': No such file or directory")
                elif node["type"] == "dir" and not recursive and node["children"]:
                    self.print_line(f"del: cannot remove '{path_arg}': Directory not empty")
                else:
                    del parent["children"][name]
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['echo']} "):
            rest = input_line[len(COMMANDS["echo"]) + 1 :]
            if " > " in rest or " >> " in rest:
                parts = rest.split()
                if ">>" in parts:
                    op_index = parts.index(">>")
                    op = ">>"
                else:
                    op_index = parts.index(">")
                    op = ">"
                message = " ".join(parts[:op_index])
                path_arg = " ".join(parts[op_index + 1 :])
                target = resolve_path(self.cwd, path_arg)
                entry = ensure_parent(self.fs, target)
                if not entry:
                    self.print_line(f"say: {path_arg}: No such file or directory")
                else:
                    parent, name = entry
                    if name not in parent["children"] or parent["children"][name]["type"] != "file":
                        parent["children"][name] = {"type": "file", "content": ""}
                    if op == ">":
                        parent["children"][name]["content"] = message + "\n"
                    else:
                        parent["children"][name]["content"] += message + "\n"
            else:
                self.print_line(rest)
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith("emod "):
            parts = input_line.split(" ")
            if len(parts) < 3:
                self.print_line("emod: usage: emod <file> <text>")
            else:
                path_arg = parts[1]
                text = " ".join(parts[2:])
                target = resolve_path(self.cwd, path_arg)
                entry = ensure_parent(self.fs, target)
                if not entry:
                    self.print_line(f"emod: {path_arg}: No such directory")
                else:
                    parent, name = entry
                    parent["children"][name] = {"type": "file", "content": text + "\n"}
                    self.print_line(f"emod: wrote {path_arg}")
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith("mod "):
            _, path_arg = input_line.split(" ", 1)
            target = resolve_path(self.cwd, path_arg)
            entry = ensure_parent(self.fs, target)
            if not entry:
                self.print_line(f"mod: {path_arg}: No such directory")
                if not from_loop:
                    self.write_prompt()
                return
            parent, name = entry
            if name not in parent["children"] or parent["children"][name]["type"] != "file":
                parent["children"][name] = {"type": "file", "content": ""}
            existing = parent["children"][name]["content"].splitlines()
            self.print_line("GNU nano 7.0  Daedalus Editor")
            self.print_line("^X Save (type .save)  ^C Cancel (type .cancel)")
            for line in existing:
                self.print_line(line)
            lines = []
            while True:
                line = sys.stdin.readline()
                if not line:
                    break
                line = line.rstrip("\n")
                if line == ".save":
                    parent["children"][name] = {"type": "file", "content": "\n".join(lines) + "\n"}
                    self.print_line("[mod] saved.")
                    break
                if line == ".cancel":
                    self.print_line("[mod] cancelled.")
                    break
                lines.append(line)
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['write']} "):
            parts = input_line.split(" ")
            if len(parts) < 3:
                self.print_line("write: usage: write <file> <text>")
            else:
                path_arg = parts[1]
                text = " ".join(parts[2:])
                target = resolve_path(self.cwd, path_arg)
                entry = ensure_parent(self.fs, target)
                if not entry:
                    self.print_line(f"write: {path_arg}: No such directory")
                else:
                    parent, name = entry
                    parent["children"][name] = {"type": "file", "content": text + "\n"}
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['grep']} "):
            parts = input_line.split(" ")
            if len(parts) < 3:
                self.print_line("find: usage: find <pattern> <file>")
            else:
                pattern = parts[1]
                path_arg = parts[2]
                target = resolve_path(self.cwd, path_arg)
                node = get_node(self.fs, target)
                if not node or node["type"] != "file":
                    self.print_line(f"find: {path_arg}: No such file")
                else:
                    for line in node["content"].split("\n"):
                        if pattern in line:
                            self.print_line(line)
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith(f"{COMMANDS['nc']} "):
            parts = input_line.split(" ")
            host = parts[1] if len(parts) > 1 else "localhost"
            port = parts[2] if len(parts) > 2 else "0"
            self.print_line(f"ping: probing {host}:{port} ... open (simulated)")
            if not from_loop:
                self.write_prompt()
            return

        if input_line == COMMANDS["sl"]:
            self.print_line(TRAIN_ART)
            if not from_loop:
                self.write_prompt()
            return

        if input_line.startswith("sys "):
            sub = input_line.split(" ")[1]
            if sub == "cpu":
                self.print_line("RP2040 CPU: Dual-core ARM Cortex-M0+, 133MHz")
            elif sub == "memory":
                self.print_line("RP2040 Memory: 264KB SRAM")
            elif sub == "info":
                self.print_line("RP2040 Info: 133MHz CPU, 264KB RAM, 2MB flash (mock)")
            else:
                self.print_line("sys [cpu|memory|info]")
            if not from_loop:
                self.write_prompt()
            return

        if input_line in {
            COMMANDS["shw"],
            COMMANDS["rmv"],
            COMMANDS["run"],
            COMMANDS["start"],
            COMMANDS["stop"],
            COMMANDS["reset"],
            COMMANDS["ports"],
            COMMANDS["peek"],
            COMMANDS["sync"],
        }:
            self.print_line(f"Mock response for {input_line}.")
            if not from_loop:
                self.write_prompt()
            return

        self.print_line(f"Unknown command: {input_line}")
        if not from_loop:
            self.write_prompt()


if __name__ == "__main__":
    cli = DaedalusCLI()
    cli.print_line("Daedalus OS booted. Type 'help' for commands.")
    cli.write_prompt()
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break
            cli.execute_command(line.strip())
        except KeyboardInterrupt:
            cli.handle_interrupt()
            cli.write_prompt()
