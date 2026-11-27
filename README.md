Yeah, in 2026 world probably doesn't need another text editor. Yet I decided to build one anyway. First, because I always wanted to and second, because I couldn't find any editor that would satisfy my humble list of wishes:

1. Works in terminal and is not restricted by legacy too much
2. Is non-modal and has vscode-like key bindings
3. Reasonably simple but powerful enough for my daily use as a text/code editor

It is inspired by Zed, Helix and micro and has the following features/traits so far:
- Multiple cursors
- Infinite undo
- Text editing behaviour and key bindings as close as possible to VS Code and Zed
- Syntax hightlight using tree-sitter for some common languages
- Zero customizability
- Underlying text buffer data structure is rope

Testing is done with a custom mini-framework of test scenarios. More on that [here](tests/scenarios/README.md).