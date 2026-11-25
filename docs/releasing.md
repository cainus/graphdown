# Releasing

Pushing a version tag publishes graphdown everywhere at once. Nothing is published from a developer machine.

## Channels

| Channel        | Install command                                   | Built by                          |
| -------------- | ------------------------------------------------- | --------------------------------- |
| GitHub Release | `curl -LsSf …/graphdown-installer.sh \| sh`       | `release.yml` (dist)              |
| npm            | `npx @cainus/graphdown check`                     | `release.yml` (dist)              |
| PyPI           | `pip install graphdown`, `uvx graphdown check`    | `publish.yml` (maturin)           |
| crates.io      | `cargo install graphdown`                         | `publish.yml`                     |
| GitHub Action  | `uses: cainus/graphdown@v0.1.0`                   | `action.yml`, using the release   |

Every channel ships the same prebuilt binary for macOS (Apple Silicon and Intel), Linux (x86_64 and arm64, glibc 2.35+) and Windows (x86_64). Nobody needs Rust installed except for `cargo install`. Why the channels look like this is recorded in @docs/design#distribution.

### GitHub Releases and npm

`dist-workspace.toml` configures dist, which generated `.github/workflows/release.yml`. On a tag it builds each target on a native runner, creates the GitHub Release with archives, checksums and the shell and PowerShell installers, then publishes `@cainus/graphdown` to npm. The npm package is a small wrapper that downloads the matching binary from the release when installed.

After changing `dist-workspace.toml`, or upgrading dist, run `dist generate` and commit the regenerated workflow. Don't edit `release.yml` by hand.

### PyPI and crates.io

`pyproject.toml` builds wheels with maturin's `bin` bindings: no Python code, just the executable placed on `PATH`. `.github/workflows/publish.yml` builds one wheel per platform plus an sdist, uploads them with PyPI trusted publishing, and runs `cargo publish`.

### The Action

`action.yml` is a composite action. It installs the release binary with the shell installer (PowerShell on Windows) and runs `graphdown check --format github`, which turns each broken reference into an annotation on the PR diff (@docs/cli#check). Pinned to a tag like `@v0.1.0`, it installs that same version; otherwise it installs the latest release, or whatever its `version` input says.

## One-time setup

These need the repository owner, once:

1. **npm:** create an automation token at npmjs.com (Access Tokens → Generate → Automation) and save it as the repo secret `NPM_TOKEN`. The `@cainus` scope is your npm username, so it exists already.
2. **PyPI:** at pypi.org, under Publishing → Add a pending publisher, enter project `graphdown`, owner `cainus`, repository `graphdown`, workflow `publish.yml`, environment `pypi`. Then create an environment named `pypi` under the repo's Settings → Environments. No token is stored anywhere.
3. **crates.io:** create an API token at crates.io (Account Settings → API Tokens) with the publish-new and publish-update scopes, and save it as the repo secret `CARGO_REGISTRY_TOKEN`.

## Cutting a release

1. Bump `version` in `Cargo.toml` and run `cargo check` so `Cargo.lock` follows.
2. Commit, then tag and push:

   ```sh
   git tag v0.1.0
   git push origin main v0.1.0
   ```

3. Watch the `release` and `publish` workflows. They are independent: a PyPI or crates.io failure does not stop the GitHub Release. Within `release`, npm publishes before the GitHub Release is announced, so a failed npm publish holds the release back. Fix the cause and re-run the failed jobs from the Actions tab rather than moving the tag.
4. After the first release, add a CI job that runs the Action against this repo with `uses: ./`, so the action is tested on every push.

A version with a suffix, like `0.2.0-rc.1`, becomes a GitHub prerelease and is not published to npm. PyPI and crates.io still receive it, as a prerelease that `pip install` and `cargo install` skip unless asked for it.
