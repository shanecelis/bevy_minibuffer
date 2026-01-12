# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

## [0.5.0] - 2026-01-12
- Update to support Bevy 0.17.
- Switch from `Event` to `Message` throughout. Events being executed
  instantaneously ended up being more difficult to manage.
- Add life-cycle app tests.
- Fix a spurious input where typing ':' for the prompt would sometimes place ':'
  as the first input character, maddening. This bug lasted over a year. The
  problem was I did not always read my keyboard messages, so sometimes one that
  was buffered from the last frame would persist to the next frame.

## [0.4.1] - 2025-04-26
- Fix [issue 3](https://github.com/shanecelis/bevy_minibuffer/issues/3) broken docs-rs build.

## [0.4.0] - 2025-04-25
- Add support for Bevy 0.16.

## [0.3.0] - 2025-01-01
- Add `TapeActs`, similar to keyboard macros.
- Add cargo feature "fun", adds sound and icon to tape acts.
- Only add one new root entity named "minibuffer".

## [0.2.0] - 2024-12-11

- Add support for Bevy 0.15.
- Add features, async, and a la carte sections to README.

## [0.1.0] - 2024-12-06

- Initial release for Bevy 0.14.
