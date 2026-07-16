# Publishing

## GitHub releases

Push a version tag and the release workflow does the rest — three-platform
builds, one addon zip, a GitHub release:

```sh
git tag v0.2.0 -m "aseprite-gd 0.2.0"
git push origin v0.2.0
```

The workflow also force-updates the `assetlib` branch: an orphan branch
holding only the built addon tree (`addons/aseprite_gd/` with binaries).
That branch exists because the Godot Asset Library installs from a git
archive of the repository, not from release assets — pointing an asset entry
at `main` would give users source code with no libraries.

## Godot Asset Library

One-time setup (requires a public repository):

1. Register an account at godotengine.org/asset-library.
2. Submit the asset once by hand: category "Tools", license MIT,
   repository URL, the `assetlib` branch's latest commit as the download
   commit, an icon URL, and the description. First submissions are reviewed
   by the library moderators.
3. After approval, note the asset id from its URL and add three repository
   secrets: `ASSETLIB_USERNAME`, `ASSETLIB_PASSWORD`, `ASSETLIB_ASSET_ID`.

From then on, every tagged release submits a version edit automatically
(the `assetlib-update` job); edits wait in the library's review queue before
going live. Without the secrets the job skips itself, so releases work fine
before the asset library is set up.
