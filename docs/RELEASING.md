# Building and releasing on GitHub

Two workflows in `.github/workflows`:

| Workflow | Runs when | What it does |
|---|---|---|
| `ci.yml` (CI) | a push to `main`, a pull request, or by hand | builds the workspace and runs its tests on Windows and Linux |
| `release.yml` (Release) | a tag `v<version>` is pushed, or by hand | tests, packages the client, and for a tag drafts a GitHub release with the archives |

## Making a release

1. Set the version: `version = "0.2.0"` under `[workspace.package]` in `Cargo.toml` (every crate takes it from there;
   the client says it in chat). Build once so that `Cargo.lock` takes the new version, and commit both.
2. Push `main` and let CI pass.
3. Tag that commit with the same version and push the tag:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. The Release workflow starts (Actions tab). It refuses a tag that is not `v` + the version in `Cargo.toml`. When it
   is through, a **draft** release is on the repository's Releases page with:
   - `ModernAnarchyOnline-0.2.0-windows-x64.zip`
   - `ModernAnarchyOnline-0.2.0-linux-x86_64.tar.gz` (when the Linux build went through)
   - `ModernAnarchyOnline-0.2.0-macos-arm64.tar.gz` (only when asked for, below)
   - `SHA256SUMS.txt`
   - notes written by GitHub from the commits since the last release, under a few lines of ours.
5. Read the draft over, edit the notes, and press **Publish**. Nothing is public before that; a draft can be deleted,
   and so can the tag (`git push origin :v0.2.0`), to start over.

A version with a dash (`0.2.0-rc.1`, tag `v0.2.0-rc.1`) is marked as a pre-release.

Run again for the same tag ("Re-run all jobs"), the workflow replaces the files of the release the tag already has and
leaves its text alone.

## What is in a package

`tools/package.ps1` (Windows) and `tools/package.sh` (Linux, macOS) make it, and the workflow only calls them, so the
same thing can be made on any machine: `.\tools\package.ps1 -Version 0.2.0` or `tools/package.sh dist 0.2.0`.

```
ModernAnarchyOnline/
  mao-client.exe      the client (the launcher when started alone); built with the `hide-console` feature, so no
                      console window opens beside it
  assets/             the bundled skins, which the client looks for beside itself
  play.bat / play.sh  finds the game folder and starts the login
  README.txt          for the player (tools/package-readme.txt)
  LICENSE             the MIT licence
```

A build for working on the client (`cargo build --release -p mao-client`) keeps its console, where the commands
print; add `--features hide-console` for one as the packages have it.

Nothing of the game is in it, and nothing from this repository's untracked files (captures, logs, `.env`) can be: the
build machines only have what is committed.

## Which systems

- **Windows x64** is the system the client is made and played on. Its build and tests have to pass, in CI and for a
  release.
- **Linux x86-64** is built and tested in both workflows, but the client has not been played there
  (`docs/PLATFORMS.md`). Its jobs may fail without failing the run or stopping a release: a release then simply has
  no Linux archive. Look at the job's log the first times.
- **macOS (Apple silicon)** is built only when asked for, because on a private repository a macOS minute counts as
  ten: tick the box when starting either workflow by hand, or set the repository variable `RELEASE_MACOS` to `true`
  (Settings, Secrets and variables, Actions, Variables) to have every tagged release build it. The program is not
  signed or notarized.

## Trying it without a release

Actions tab, **Release**, **Run workflow**, pick the branch. It builds the same packages, named
`ModernAnarchyOnline-<version>-dev.<commit>-...`, and keeps them with the run for two weeks under "Artifacts". No tag,
no release.

## The tests on the build machines

- No game is installed there. The tests that read the game's files look for `MAO_GAME` and skip themselves without
  it, so CI checks the code that needs no game data (parsers on built-in samples, the protocol, the interface's
  logic). The full run stays a local one: `MAO_GAME=<game folder> cargo test --release --workspace`.
- The render tests take whatever graphics device the machine has. A build machine has only the system's software
  device (or none, and then they skip themselves). `MAO_SOFTWARE_GPU=1` makes the tests use the software device on a
  machine that has a real one, to see what they do there.
- No account, no network login: nothing in the tests logs in.

## Minutes (a private repository)

GitHub counts a Windows minute as two and a macOS minute as ten against the included minutes. With the compiled
dependencies kept between runs (`Swatinem/rust-cache`, shared by both workflows) a CI run is the workspace's own
crates and the tests. To spend less: pushes that only touch `docs/`, `*.md` or `tools/*.py` start no CI run already;
a newer push stops the run of an older one; and the `push:` block of `ci.yml` can be removed to leave CI to pull
requests and runs started by hand.

## What the workflows use

- `actions/checkout`, `actions/upload-artifact`, `actions/download-artifact` (GitHub's own) and
  `Swatinem/rust-cache` (the dependency cache). No other outside action; the release itself is made with the `gh`
  program that the build machines have, with the run's own token (`contents: write` for that one job only).
- Rust `stable`, as installed by `rustup` on the machine at the time. To pin it, put a `rust-toolchain.toml` in the
  repository's root; `rustup` then takes that version everywhere, the workflows included.
- `--locked` everywhere: a build that would change `Cargo.lock` fails instead, so what is released is what was
  committed.
