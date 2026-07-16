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

## Godot Asset Store (store.godotengine.org)

The official store accepts direct file uploads — the release zip is the
exact artifact it wants. Per release:

1. Sign in and add a new version to the asset (or create the asset once at
   `/asset/new/`: category, MIT license, description, icon, screenshots).
2. Upload `aseprite-gd-<tag>.zip` from the GitHub release.

There is no published automation API yet; uploads are manual. The store is
integrated into the editor from Godot 4.7.

## Legacy Asset Library (optional)

The pre-4.7 editor AssetLib tab uses the older godotengine.org/asset-library,
which is deprecated (soon read-only) and installs from git archives rather
than uploads. If listing there anyway:

1. Register an account and submit once by hand, pointing the download at the
   `assetlib` branch's latest commit (that orphan branch carries the built
   addon tree exactly for this purpose; the release workflow refreshes it).
2. After approval, add repository secrets `ASSETLIB_USERNAME`,
   `ASSETLIB_PASSWORD`, and `ASSETLIB_ASSET_ID`; each tagged release then
   files a version edit automatically. Without the secrets that job skips
   itself.
