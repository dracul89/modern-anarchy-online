# Building and releasing on GitHub

Two workflows in `.github/workflows`:

| Workflow | Runs when | What it does |
|---|---|---|
| `ci.yml` (CI) | a push to `main`, a pull request, or by hand | builds the workspace and runs its tests on Windows and Linux; builds it on macOS |
| `release.yml` (Release) | a tag `v<version>` is pushed, or by hand | tests, packages the client, and for a tag drafts a GitHub release with the archives |

## Making a release

1. Set the version: `version = "0.2.0"` under `[workspace.package]` in `Cargo.toml` (every crate takes it from there;
   the client says it in chat). Build once so that `Cargo.lock` takes the new version. Write the version's section
   in `CHANGELOG.md` (`## 0.2.0 (date)`, what changed as a player meets it). Commit the three.
2. Push `main` and let CI pass.
3. Tag that commit with the same version and push the tag:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. The Release workflow starts (Actions tab). It refuses a tag that is not `v` + the version in `Cargo.toml`. When it
   is through, a **draft** release is on the repository's Releases page with:
   - `ModernAnarchyOnline-0.2.0-windows-x64.zip`
   - `ModernAnarchyOnline-0.2.0-windows-x64-setup.exe`: an installer (Inno Setup, `tools/installer.iss`) with a Start
     menu entry, an optional desktop shortcut and an uninstaller; for one user unless asked otherwise; not signed
   - `ModernAnarchyOnline-0.2.0-windows-arm64.zip` and `...-windows-arm64-setup.exe` (when the ARM64 build went
     through): the same two for Windows on ARM64 processors; that installer installs on ARM64 machines only
   - `ModernAnarchyOnline-0.2.0-linux-x86_64.tar.gz` (when the Linux build went through)
   - `ModernAnarchyOnline-0.2.0-linux-x86_64.AppImage`, with it: the client, its skins, icon and menu entry in one
     file to make executable and run (`tools/package-appimage.sh`; the system's own glibc, ALSA and Vulkan are used;
     both Linux files need glibc 2.35 or newer, the build machine being Ubuntu 22.04)
   - `ModernAnarchyOnline-0.2.0-macos-arm64.tar.gz` (while the repository variable `RELEASE_MACOS` is `true`, below)
   - `ModernAnarchyOnline-0.2.0-macos-arm64.dmg`, with it: the application (`Modern Anarchy Online.app`, icon and
     all) in a disk image to drag onto Applications, made by `tools/package-macos.sh`; signed with the
     Developer ID certificate and notarized while the repository has the signing secrets ("Signing the macOS
     application", below), else signed ad hoc
   - `SHA256SUMS.txt`
   - notes: a few lines of ours, then the version's section of `CHANGELOG.md` as the tag has it ("What is new in
     0.2.0"; left out when the file has no such section), then GitHub's link to the commits since the last release.
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
- **Windows ARM64** (`aarch64-pc-windows-msvc`) is built in the Release workflow only, on the x64 machine for the
  other processor (`tools/package.ps1 -Target aarch64-pc-windows-msvc`, `iscc /DArch=arm64`), so its tests are not
  run and the client has not been started there. Its job may fail without stopping a release. An ARM64 machine also
  runs the x64 package, under Windows' emulation.
- **Linux x86-64** is built and tested in both workflows, but the client has not been played there
  (`docs/PLATFORMS.md`). Its jobs may fail without failing the run or stopping a release: a release then simply has
  no Linux archive. Look at the job's log the first times.
- **macOS (Apple silicon)** is built in every CI run, without its tests (the box of a run started by hand adds
  them), and its job may fail without failing the run. Every tagged release packages it while the repository variable
  `RELEASE_MACOS` is `true` (Settings, Secrets and variables, Actions, Variables), as it is set now; a Release run
  started by hand packages it when the box is ticked. On a private repository a macOS minute counts as ten: there,
  clear the variable and take the macOS entry out of `ci.yml`'s matrix.

## Signing the macOS application

`tools/package-macos.sh` signs the application and the disk image with a Developer ID Application certificate when
the keychain has one, and has Apple notarize the image when `MAO_NOTARY_PROFILE` names stored notary credentials
(the top of the script says how, for a Mac of one's own). A notarized image opens on another Mac without the
system's warning; one signed ad hoc does not.

The Release workflow does the same when the repository has these five secrets (Settings, Secrets and variables,
Actions, Secrets); without `MACOS_CERTIFICATE_P12` it signs ad hoc and says so in its log:

| Secret | What |
|---|---|
| `MACOS_CERTIFICATE_P12` | the Developer ID Application certificate with its private key, exported from Keychain Access as a `.p12` (My Certificates, the certificate, Export) and base64-encoded |
| `MACOS_CERTIFICATE_PASSWORD` | the password given to that export |
| `MACOS_NOTARY_APPLE_ID` | the Apple account's address |
| `MACOS_NOTARY_TEAM_ID` | the team's ten characters |
| `MACOS_NOTARY_PASSWORD` | an app-specific password of that account (account.apple.com, Sign-In and Security) |

```sh
base64 -i DeveloperID.p12 | gh secret set MACOS_CERTIFICATE_P12
gh secret set MACOS_CERTIFICATE_PASSWORD      # (asks for the value; so do the other three)
```

The certificate is put into a keychain of the run's own after the client is built, and the machine is thrown away
with the run. Pull requests never see the secrets: the Release workflow runs for tags and by hand only. With a wrong
certificate or a refused notarization the macOS job fails and the release has no macOS files; with the secrets
absent, a notarized image made on a Mac can still replace the draft's, with its line in `SHA256SUMS.txt`, before
the draft is published (a published release's files cannot be changed while the repository keeps its releases
immutable). The present certificate runs out on 15 December 2027 (a new one is then exported and the two certificate secrets set again) and the `.tar.gz` for a terminal is not signed this way.

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

## Dependency alerts

- **tokio** (2026-11-14): 1.54.0 to 1.54.1 in `Cargo.lock`. It is in the lock for the web target only (through `wasm-bindgen-futures`).
- **rand, affected 0.7.0 up to 0.8.6** (2026-11-14): dismissed as not used. The lock's rand 0.7.3 comes only through `wry` 0.53.5's Android-only HTML parser (`kuchikiki`, whose `selectors` generates code with `phf_codegen` 0.8 at build time); no platform this project builds compiles it. `wry` 0.54.4 drops that parser and builds here, but a launcher test with it misbehaved and the bump was put back: it wants a careful trial of the launcher and the in-game pages first.
