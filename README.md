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

```rust no_run
//! Add a command.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_world() {
    info!("Hello, world");
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_acts(Act::new(hello_world))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
```

- [x] Easily bind key chord sequences to acts via [bevy-input-sequences](https://github.com/not-elm/bevy-input-sequence)

```rust no_run
//! Add a command with a hotkey.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_world() {
    info!("Hello, world");
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_acts(Act::new(hello_world).hotkey(keyseq! { Ctrl-H }))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
```

- [x] Easily solicit user for input via [bevy_asky](https://github.com/shanecelis/bevy_asky)

```rust no_run
//! Ask user a question.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.prompt::<TextField>("What's your name? ")
        .observe(|trigger: AskyEvent<String>| {
            info!("Hello, {}", trigger.event().unwrap());
        });
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .add_systems(Startup, hello_name)
        .run();
}
```
- [x] Tab completion where possible
- [x] Easily opt-in to built-in functionality
- [ ] Easily exclude from build

I believe a project with a "minibuffer" feature flag and rust conditional
compilation facilities ought to make it easy and practical to exclude from a
release build. But I'd like to affirm that in practice before checking that box.

# Antigoals

- No general-purpose text editing

We are not making a text editor.

- No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It
can resize to accommodate more than one-line of text.

- No default kitchen sink

The default functionality should be a blank slate that does nothing if no
commands or key bindings have been added. Built-in functions like `exec_act` and
the ":" key binding should be opt-in.

# FAQ

## Why are Minibuffer commands called acts?

Bevy has a foundational trait called `Command`. Minibuffer's commands are called
`Act`s to avoid the confusion of having two very different `Command` types.

## Why not a shell?

If one surveys developer consoles, one will find that many have taken
inspiration from command-line interfaces, Unix shells being the most prevalent.
And the Unix shell is great; I love it and use it daily. However, I do not
believe it represents the best interaction model for game developer consoles.

A non-interactive Unix command requires one to provide the arguments it expects.
Failure to do so results in an error message. Often one must consult the
command's help or usage to determine the right arguments. We tolerate this
because we can then script these interactions.

In general the Unix shell trades interactive convenience for non-interactive
scriptability, and it is a good trade because of its scriptability. Minibuffer
does not provide interactive scriptability[^1] but that means we can make it a
better interactive experience. For instance instead of being required to know
the arguments for any given command, Minibuffer commands will query the user for
what they require. It is a "pull" model of interaction versus a "push" model.

[^1:] Although one could implement keyboard macros. PRs are welcome.

# TODO
- [ ] Use a real cursor/selection highlight that doesn't [fail on wrap](https://discord.com/channels/691052431525675048/1305257817057398825/1305257817057398825).
- [x] Change the keyseq macros to capitalize modifiers like "Ctrl-C" instead of "ctrl-C".
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
