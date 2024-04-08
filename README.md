# bevy_minibuffer

This is a developer console for the bevy game engine. It's inspired by the
user interface of classic unix text editors rather than the unix shell.

> [!CAUTION]
> `bevy_minibuffer` is currently in the early stages of development and is subject to breaking changes.

# Goals

- Easily add commands
- Easily bind key chord sequences to commands
- Easily solicit user for textual input
- Tab completable
## Unrealized goals
- Easily exclude from build
- Easily opt-in to built-in functionality

# Antigoals

- No general text editing
- No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It can resize to accommodate more than one-line of text. 

- No default kitchen sink

The default functionality should be a blank slate that does nothing if no commands or key bindings have been added. Built-in functions like `exec_act` and the ":" key binding should be opt-in.

# FAQ

## Why are bevy_minibuffer commands called acts?

`bevy_minibuffer` commands are called `Act`s to avoid confusion because bevy
already has its own `Command` struct.

# Design Quetions
## Re: No windows antigoal
The minibuffer can show more than one line of text, but what to do if its asked
to show multiple pages of text?

# License

This crate is licensed under the MIT License or the Apache License 2.0.
