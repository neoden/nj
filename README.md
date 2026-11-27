Yeah, in 2026 world probably doesn't need another text editor. Yet I decided to build one anyway. First, because I always wanted to and second, because I couldn't find any editor that would satisfy my humble list of wishes:

1. Works in terminal and is not restricted by legacy too much
2. Is non-modal and has vscode-like key bindings
3. Reasonably simple but powerful enough for my daily use as a text/code editor

And I wanted to have a playground to test some of my ideas about what a post-post-modern text editor could look like.

It is inspired by Zed, Helix and micro and has the following features/traits so far:
- Expects a modern terminal emulator that supports kitty protocol
- Multiple cursors
- Infinite undo
- Text editing behaviour and key bindings as close as possible to VS Code and Zed
- Syntax highlight using tree-sitter for some common languages
- Zero customizability
- Underlying text buffer data structure is rope

Testing is done with a custom mini-framework of test scenarios. More on that [here](tests/scenarios/README.md). Testing is important to be done properly, because the code is of course written by AI. So this framework tries to make it convenient to create and run end-to-end testing scenarios against a live instance of the editor. Also, in attempt to address the way AI agents are so bad in writing maintainable and performant code, these scenarios allow making asserts about execution time and memory usage of certain operations in terms of assymptotic complexity.
