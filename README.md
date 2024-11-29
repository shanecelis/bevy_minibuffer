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
## Easily opt-in to built-in functionality

``` rust ignore
fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
       .add_acts(Builtin::default());
}
```

Adding the `Builtin` acts provides the following:

| ACT               | KEY BINDING |
|-------------------|-------------|
| describe_key      | Ctrl-H K    |
| exec_act          | :<br>Alt-X  |
| list_acts         | Ctrl-H A    |
| list_key_bindings | Ctrl-H B    |
| toggle_visibility | `           |

``` sh
cargo run --example opt-in
```
## Easily add acts, i.e., commands

```rust ignore 
fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
}

fn plugin(app: &mut App) {
    app.add_acts((Act::new(hello_world), Builtin::default()));
}
```

Acts are systems. Any system will do.

NOTE: We add `Builtin` acts here only because there would be no way to run an act without a key binding.
``` sh
cargo run --example add-act
```

## Easily bind key chord sequences to acts 

```rust ignore
fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
    minibuffer.set_visible(true);
}

fn plugin(app: &mut App) {
    app.add_acts(Act::new(hello_world).bind(keyseq! { Ctrl-H }));
}
```
``` sh
cargo run --example bind-hotkey
```
## Easily solicit user for input 

```rust ignore fn hello_name(mut minibuffer: Minibuffer) { minibuffer.prompt::<TextField>("What's your name? ") .observe(|mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| { minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap())); }); }

fn plugin(app: &mut App) {
    app.add_systems(PostStartup, hello_name);
}
```
``` sh
cargo run --example solicit-user
```
## Tab completion where possible
``` rust ignore
fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.read("What's your name? ",
                    vec!["John", "Sean", "Shane"])
        .observe(|mut trigger: Trigger<Submit<String>>, 
                  mut minibuffer: Minibuffer| {
            minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap()));
        });
}

fn plugin(app: &mut App) {
    app.add_systems(PostStartup, hello_name);
}
```
``` sh
cargo run --example tab-completion
```
## Easily exclude from build

I believe a project with a "minibuffer" feature flag and rust conditional
compilation facilities ought to make it easy and practical to exclude it from a
release build. But I'd like to affirm that in practice before checking that box.

# Antigoals

## No general-purpose text editing

We are not making a text editor.

## No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It
can resize to accommodate more than one-line of text.

## No default kitchen sink

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
