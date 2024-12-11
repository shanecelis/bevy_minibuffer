# bevy_minibuffer

This is a developer console for the [Bevy game engine](https://bevyengine.org).
It is inspired by the user interface of classic Unix text editors rather than
the Unix shell.

> [!CAUTION]
> `bevy_minibuffer` is currently in the early stages of development and is subject to breaking changes.

# Example
<p align="center">
  <img src="https://github.com/user-attachments/assets/8d8dc5cf-b20c-4d8d-97f4-de8fdf176a24"/>
</p>

The video above shows the [demo-async](/examples/async/demo.rs) example.

```sh
cargo run --example demo-async --features async
```

# Goals
- Easily opt-in to basic functionality
- Easily add acts, i.e., commands
- Easily bind key chord sequences to acts 
- Easily solicit user for input 
- Tab completion where possible
- Easily exclude from build

# Antigoals
- No default kitchen sink

The default functionality should be a blank slate that does nothing if no
commands or key bindings have been added. Basic functions like `run_act` and
the ":" key binding should be opt-in.
- No general-purpose text editing
- No windows or panels

Try to force everything through the minibuffer at the bottom of the screen. It
can resize to accommodate more than one-line of text.

# Examples
An example for every goal.

## Easily opt-in to basic functionality
<img align="right" src="https://github.com/user-attachments/assets/0e5e77a2-7c91-4660-8962-bb356d91bf98"/>

`MinibufferPlugins` does not include any built-in acts or key bindings, but it is
expected that many users will want some kind of basic functionality. `BasicActs`
provides the following acts and key bindings:

| ACT               | KEY BINDING |
|-------------------|-------------|
| describe_key      | Ctrl-H K    |
| run_act           | :<br>Alt-X  |
| list_acts         | Ctrl-H A    |
| list_key_bindings | Ctrl-H B    |
| toggle_visibility | `           |

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
       .add_acts(BasicActs::default());
}
```

``` sh
cargo run --example opt-in
```
## Easily add acts, i.e., commands
<img align="right" src="https://github.com/user-attachments/assets/d7e1ec10-787b-4ce1-98c0-63960df4e435"/>

Acts are systems. Any system[^1] will do.

NOTE: We add `BasicActs` acts here only because there would be no way to run an
act otherwise. To run an act without `BasicActs`, one would need a key binding.

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
}

fn plugin(app: &mut App) {
    app.add_acts((Act::new(hello_world), 
                  BasicActs::default()));
}
```

``` sh
cargo run --example add-act
```

[^1]: Any system with no input or output. This does not exclude pipelines, however,
  which are used extensively with asynchronous systems.

## Easily bind key chord sequences to acts 
<img align="right" src="https://github.com/user-attachments/assets/336a79c1-f934-4d69-a3fe-6b55778663be"/>
We can bind key chord `Ctrl-W` or even a key chord sequence `Ctrl-W Alt-O Super-R Shift-L D` to an act.

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
    minibuffer.set_visible(true);
}

fn plugin(app: &mut App) {
    app.add_acts(Act::new(hello_world)
                 .bind(keyseq! { Ctrl-W }));
}
```
``` sh
cargo run --example bind-hotkey
```
## Easily solicit user for input 
<img align="right" src="https://github.com/user-attachments/assets/03cbb697-8263-41cb-b40f-583d1a25d429"/>
Ask the user for information. 

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
fn hello_name(mut minibuffer: Minibuffer) {
  minibuffer
    .prompt::<TextField>("What's your name? ")
    .observe(|mut trigger: Trigger<Submit<String>>, 
              mut minibuffer: Minibuffer| {
        minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap()));
    });
}

fn plugin(app: &mut App) {
    app.add_systems(Startup, 
                    hello_name);
}
```
``` sh
cargo run --example solicit-user
```

Minibuffer supports the following prompts:
- Checkboxes
- Confirm
- Numbers
  - u8, u16, u32, u64, i*, f*, usize, isize
- Radio buttons
- Toggle
- TextField
  - Tab completion

See the "demo-async" example to see more prompts in action.
``` sh
cargo run --example demo-async --features=async
```

## Tab completion where possible
Text centric user interfaces ought to support tab completion where possible. 

### Use a `Vec`
<img align="right" src="https://github.com/user-attachments/assets/8b2b8a13-1ee3-4341-b523-c33fa80d4be2"/>

One can provide a list of strings for simple completions. 

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.prompt_lookup("What's your name? ",
                             vec!["John", "Sean", "Shane"])
        .observe(|mut trigger: Trigger<Submit<String>>, 
                  mut minibuffer: Minibuffer| {
            minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap()));
        });
}

fn plugin(app: &mut App) {
    app.add_systems(Startup, hello_name);
}
```
``` sh
cargo run --example tab-completion vec
```

### Use a `Trie`
One can provide a trie for more performant completion. 

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
# use trie_rs::Trie;
fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.prompt_lookup("What's your name? ",
                             Trie::from_iter(["John", "Sean", "Shane"]))
        .observe(|mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
            minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap()));
        });
}
```
``` sh
cargo run --example tab-completion trie
```

### Use a `HashMap`
One can provide a hash map that will provide completions and mapping to values.

``` rust no_run
#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare,
}

fn hello_name_hash_map(mut minibuffer: Minibuffer) {
    let map = HashMap::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    minibuffer.prompt_map("What's your name? ", map).observe(
        |mut trigger: Trigger<Completed<Popular>>, mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result().unwrap();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's a {:?} name.", popular),
                _ => "I don't know what kind of name that is.".to_string(),
            });
        },
    );
}
```

### Use a `map::Trie`
<img align="right" src="https://github.com/user-attachments/assets/af7b33e0-135d-4d6c-b748-a489a1245000"/>

One can provide a trie that maps to an arbitary value type `V` and receive the
value `V` type in response in addition to the string. This is more performant
than a hash map.

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;
# use trie_rs::map::Trie;
#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare
}

fn hello_name(mut minibuffer: Minibuffer) {
    let trie = Trie::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    minibuffer.prompt_map("What's your name? ", trie).observe(
        |mut trigger: Trigger<Completed<Popular>>, 
         mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result().unwrap();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's a {:?} name.", popular),
                _ => "I don't know what kind of name that is.".into(),
            });
        },
    );
}
```
``` sh
cargo run --example tab-completion trie-map
```
## Easily exclude from build

I _believe_ a project with a "minibuffer" feature flag and rust conditional
compilation facilities ought to make it easy and practical to exclude it from a
release build. But I'd like to affirm that in practice before considering this goal achieved.

``` rust ignore
#[cfg(feature = "minibuffer")]
fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
       .add_acts(BasicActs::default());
}
 
```

# Async

An "async" feature flag makes the `MinibufferAsync` system parameter available.
Unlike the regular `Minibuffer` system parameter, `MinibufferAsync` can be
captured by closures.

Although one can technically achieve the same behavior with `Minibuffer`, there
are cases like those with many queries in succession where using
`MinibufferAsync` is more expressive. 

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt::<TextField>("What's your first name? ")
        .await?;
    let last_name = minibuffer
        .prompt::<TextField>("What's your last name? ")
        .await?;
    minibuffer.message(format!("Hello, {first_name} {last_name}!"));
    Ok(())
}

fn plugin(app: &mut App) {
    app.add_acts(ask_name.pipe(future_result_sink));
}
```
The preceding async function `ask_name()` returns a future, technically a `impl Future<Output
= Result<(), Error>>`. That has to go somewhere so that it will be evaluated.
There are a series of functions in the `sink` module:

- `future_sink` accepts any future and runs it.
- `future_result_sink` accepts any future that returns a result and runs it but
  on return if it delivered an error, it reports that error to the minibuffer.

# Acts and Plugins

An `ActsPlugin` is a `Plugin` that contains `Act`s. Two `ActsPlugin`s are
available in this crate: `BasicActs` and `UniversalArgActs`.

## BasicActs

`BasicActs` has the bare necessities of acts: 
- run_act

Asks for what act to run, provides tab completion.
- list_acts

Lists acts and their key bindings.
- list_key_bindings

Lists key bindings and their acts.
- describe_key

Listens for key chords and reveals what act they would run.
- toggle_visibility

Hides and shows the minibuffer.

But one can trim it down further if one likes by calling `take_acts()`,
manipulating them, and submitting that to `add_acts()`. For instance to only add
'run_act', one would do the following:

``` rust no_run
# use bevy::prelude::*;
# use bevy_minibuffer::prelude::*;

fn plugin(app: &mut App) {
    let mut basic_acts = BasicActs::default();
    // Acts is a HashMap of act names and [ActBuilder]s.
    let mut acts = basic_acts.take_acts();
    // `basic_acts` no longer has any acts in it. We took them.
    let list_acts = acts.remove("list_acts").unwrap();
    app.add_plugins(MinibufferPlugins)
        .add_acts((basic_acts, // Or one could do: `.add_plugins(basic_acts)`.
                   list_acts));
}
```

## `UniversalArgActs`
<img align="right" src="https://github.com/user-attachments/assets/a227b529-ba66-403d-a984-5f4c7ac1d5b2"/>

Provides a univeral argument that acts can use by accessing the
`Res<UniveralArg>`. It simply holds an option of a signed number.

``` rust ignore
pub struct UniversalArg(pub Option<i32>);
```

One uses it like so, type `Ctrl-U 1 0` and this would place 10 into the
`UniversalArg` resource. It is cleared after the next act runs. See the example.

``` sh
cargo run --example universal-arg --features async
```

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
command's help or usage to determine the right arguments. This is
tolerable partly because we can then script these interactions.

In general the Unix shell trades interactive convenience for non-interactive
scriptability, and it is a good trade because of its scriptability. Minibuffer
does not provide interactive scriptability[^2] but that means we can make it a
better interactive experience. For instance instead of being required to know
the arguments for any given command, Minibuffer commands will query the user for
what is required. It is a "pull" model of interaction versus a "push" model.

[^2]: Although one could implement keyboard macros, which are a form of interactive scripting. Pull requests are welcome.

# TODO
- [ ] Make it possible to have vim-like prompt (no space after ":").
- [ ] Use a "real" cursor/selection highlight.
- [x] Add `HashMap<String,V>` completer.
- [x] Make universal-arg work without async.
- [x] Re-write [asky](https://github.com/axelvc/asky) to be [bevy native](https://github.com/shanecelis/bevy_asky).

# Design Questions
## Re: No windows antigoal
The minibuffer can show more than one line of text, but what to do if its asked
to show multiple pages of text?

This is an unresolved issue.

# Compatibility

| bevy_minibuffer | bevy |
|-----------------|------|
| 0.1.0           | 0.14 |

# License

This crate is licensed under the MIT License or the Apache License 2.0.
