# Release

1. Update `CHANGELOG.md` with the new version and date
2. Bump the version in `Cargo.toml`
3. Run `cargo check` to update `Cargo.lock`
4. Commit: `git commit -am "Release v0.X.0"`
5. Tag: `git tag v0.X.0`
6. Push: `git push && git push --tags`

Pushing the tag triggers the [release workflow](.github/workflows/release.yml),
which uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) to build
binaries for all targets and create a GitHub Release.

## Upgrading dist

`release.yml` is generated from `dist-workspace.toml`, so the version pin can't
be bumped by editing either file — `dist plan` fails any PR that does. Let dist
do it:

```sh
dist selfupdate --version X.Y.Z --yes  # updates the pin and regenerates release.yml
dist plan                              # sanity check
```

A [scheduled workflow](.github/workflows/cargo-dist-update.yml) opens an issue
when a new release is available.
