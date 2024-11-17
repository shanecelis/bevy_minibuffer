# bevy_minibuffer

This is a developer console for the bevy game engine. It's inspired by the
user interface of classic unix text editors rather than the unix shell.

> [!CAUTION]
> `bevy_minibuffer` is currently in the early stages of development and is subject to breaking changes.

# Example
![two_commands example](https://github.com/shanecelis/bevy_minibuffer/assets/54390/e90c4ef9-664c-47af-8ff5-a83214237341)

The video above shows the [two_commands.rs](examples/two_commands.rs) example.

# Goals

- Easily add commands
- Easily bind key chord sequences to commands
- Easily solicit user for textual input
- Tab completable
## Unrealized goals
- Easily exclude from build
- Easily opt-in to built-in functionality

# Antigoals

- No general-purpose text editing
- No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It can resize to accommodate more than one-line of text. 

- No default kitchen sink

The default functionality should be a blank slate that does nothing if no commands or key bindings have been added. Built-in functions like `exec_act` and the ":" key binding should be opt-in.

# FAQ

## Why are bevy_minibuffer commands called acts?

`bevy_minibuffer` commands are called `Act`s to avoid confusion because bevy
already has its own `Command` struct.

# TODO
- [ ] Use a real cursor/selection highlight that doesn't [fail on wrap](https://discord.com/channels/691052431525675048/1305257817057398825/1305257817057398825).
- [ ] Change the keyseq macros to use lower case, or use caps on mods like "Ctrl-C".
- [ ] Copy-and-paste the color::View to create Minibuffer's own View.
- [ ] Get off of unreleased dependencies.
- [x] Re-write asky to be bevy native.

# Design Quetions
## Re: No windows antigoal
The minibuffer can show more than one line of text, but what to do if its asked
to show multiple pages of text?

# License

This crate is licensed under the MIT License or the Apache License 2.0.
