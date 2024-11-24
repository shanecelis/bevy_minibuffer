# bevy_minibuffer

This is a developer console for the [Bevy game engine](https://bevyengine.org).
It is inspired by the user interface of classic Unix text editors rather than
the Unix shell.

> [!CAUTION]
> `bevy_minibuffer` is currently in the early stages of development and is subject to breaking changes.

# Example
![two_commands example](https://github.com/shanecelis/bevy_minibuffer/assets/54390/e90c4ef9-664c-47af-8ff5-a83214237341)

The video above shows the [two_commands.rs](examples/two_commands.rs) example.

# Goals

- [x] Easily add acts, i.e., commands
- [x] Easily bind key chord sequences to acts via [bevy-input-sequences](https://github.com/not-elm/bevy-input-sequence)
- [x] Easily solicit user for input via [bevy_asky](https://github.com/shanecelis/bevy_asky)
- [x] Tab completion where possible
- [x] Easily opt-in to built-in functionality
- [ ] Easily exclude from build

I believe a project with a "minibuffer" feature flag and rust conditional
compilation facilities ought to make it easy to exclude from a release build.
But I'd like to affirm that in practice before checking that box.

# Antigoals

- No general-purpose text editing
- No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It
can resize to accommodate more than one-line of text.

- No default kitchen sink

The default functionality should be a blank slate that does nothing if no
commands or key bindings have been added. Built-in functions like `exec_act` and
the ":" key binding should be opt-in.

# FAQ

## Why are Minibuffer commands called acts?

Bevy has a foundational trait called `Command`. Calling Minibuffer's commands
`Act`s is to avoid confusing the two.

# TODO
- [ ] Use a real cursor/selection highlight that doesn't [fail on wrap](https://discord.com/channels/691052431525675048/1305257817057398825/1305257817057398825).
- [ ] Change the keyseq macros to capitalize modifiers like "Ctrl-C" instead of "ctrl-C".
- [x] Copy-and-paste the color::View to create Minibuffer's own View.
- [ ] Get off of unreleased dependencies.
- [x] Re-write asky to be bevy native.

# Design Quetions
## Re: No windows antigoal
The minibuffer can show more than one line of text, but what to do if its asked
to show multiple pages of text?

This is an unresolved issue.

# License

This crate is licensed under the MIT License or the Apache License 2.0.
