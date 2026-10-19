# Maintainer

## Pages

WASM builds of every example are published [here](https://shanecelis.github.io/bevy_minibuffer/).
Publish that site from this machine with `cargo make pages`.
`cargo make pages` builds every example, assembles `web/dist`, and force-pushes one commit to the `gh-pages` branch.
The Pages workflow is gone, so a push to `main` leaves the site as it is.
`cargo make pages-build-one opt-in` builds a single example.
`cargo make pages-dist pages-push` republishes the `web/dist` already on disk.
You need `cargo install cargo-make`, `trunk`, and `rustup target add wasm32-unknown-unknown`.

GitHub Pages has to serve the `gh-pages` branch at `/`. After the first push:

```sh
gh api -X PUT repos/shanecelis/bevy_minibuffer/pages \
  -f build_type=legacy \
  -f 'source[branch]=gh-pages' \
  -f 'source[path]=/'
```

If GitHub still has the Actions source selected, switch it once under Settings → Pages.
Later publishes only update the `gh-pages` branch.
