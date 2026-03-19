#Daedalus OS (previously flux)

#Daedalus is a rebuilt version of my previous OS and kernel, so I use snippets of code from there

Daedalus OS is a web-based OS simulator with a terminal-first interface and a toggleable desktop view. It runs entirely in the browser and includes a simulated file system plus a command parser that supports loops, interrupts, and custom commands.

What this repo includes

- index.html: The full web experience (terminal, desktop UI, simulated file system).
- daedalus_cli.py: A local terminal CLI that mirrors the web shell commands and file system.

Commands

Type help in the shell to see all commands and usage. The CLI and the web version share the same commands.

About the upcoming builds

- Assembly build: There is an assembly version that can run directly on a Raspberry Pi Pico as a simple kernel. It is not in this GitHub repo because it is still unstable. It will be ready in a few days.
- Local desktop kernel: There is also a local desktop kernel you can boot from. It is also unstable and not included yet. It will be ready in a few days.

For now, use the Python CLI in this repo for local use.

GitHub Pages

This project is GitHub Pages friendly. Deploy from the main branch and use the repository root.

Local CLI

Run this in your terminal:

python3 daedalus_cli.py
