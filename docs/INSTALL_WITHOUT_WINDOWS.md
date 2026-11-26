# Getting the game's files without Funcom's Windows installer

The client needs an installed Anarchy Online folder (`cd_image`, `Setupf`, `version.id`). Funcom's installer and its
patcher are Windows programs. `mao-client install` and `mao-client patch` do what those two do, in Rust, on every
system the client builds for. This is how to use them, how each format was read from Funcom's programs (nothing of
theirs was run), and what was checked.

```
mao-client install <folder> [--accept-eula] [--downloads <folder>] [--keep-downloads] [--allow-unknown-installer]
mao-client patch <folder>
```

`install` downloads Funcom's installer from Funcom's servers (2.1 GB), shows Funcom's licence and asks, unpacks the
game into the folder (7,098 files, 2.9 GB) and patches it to the version the servers run (12 steps today, 27 MB).
Nothing downloaded is run. A run that was cut short goes on where it stopped: unfinished downloads are continued, a
folder that already holds the unpacked game is only patched, and a patch step that was already applied is passed
over. The installer's parts are kept in `<folder>/mao-installer` while it works and deleted at the end (`--downloads`
names another place and keeps them). `patch` is the second half alone, for a later patch of Funcom's or for a folder
copied over from Windows.

Nothing of Funcom's is in the repository or in a release: the files are fetched from Funcom's servers on the player's
machine, the same files Funcom's own installer and patcher fetch. The code is the crate `mao-install`
(`fetch`, `inno`, `rtpatch`, `respatch`, `ctree`, `crypto`, `patcher`, `install`); the two commands are
`crates/mao-client/src/installcmd.rs`.

Sources read: Funcom's download `AnarchyOnline_EP1.exe` (444,632 bytes, SHA-256 `ed5ff785…607c9e`); the installer it
fetches; an installed game patched to 18.8.62; `AnarchyPatcher.exe` 1.0.3.1 (linked 2023-06-02) and `PATCHW32.DLL`
(RTPatch 5.21), decompiled with Ghidra. Addresses below are those programs' own.

## 1. Funcom's download is a downloader

`AnarchyOnline_EP1.exe` is an **NSIS 2.46** installer (linked 2009-12-05) with no game data in it. After the PE image
come 37,592 bytes: NSIS's first header (`0xDEADBEEF`, `NullsoftInst`) and five raw-deflate blocks, the script (36,585
bytes unpacked), the wizard bitmap, and the plug-ins `nsDialogs.dll`, `System.dll` and `NSISdl.dll`. The script, decoded
(540 instructions):

1. shows Funcom's licence (the text is in the script) and asks for a folder;
2. downloads `http://cdn-web-nj4.funcom.com/download/AO/current_EP1.txt` into it as `current.txt`;
3. reads that file line by line, each line three quoted words: `"dl" "<address>" "<file name>"` downloads the address
   to the file with `NSISdl::download`; `"in" "<file name>"` runs the file.

`current_EP1.txt` today (586 bytes, last changed 2020-06-17):

| File | Bytes |
|---|---|
| `http://update.anarchy-online.com/download/AO/18.8.50_EP1/install.exe` | 964,912 |
| `…/install-1.bin` | 523,336,192 |
| `…/install-2.bin` | 524,288,000 |
| `…/install-3.bin` | 524,288,000 |
| `…/install-4.bin` | 524,288,000 |
| `…/install-5.bin` | 705,275 |
| then `"in" "install.exe"` | 2,097,870,379 in all |

`fetch.rs` reads the list the same way (`installer_list`) and downloads the parts with `Range` requests, so a download
goes on from where it was cut (the server answers `Accept-Ranges: bytes`). There is no newer full installer on the
server (`18.8.62_EP1/install.exe` is not there).

The servers speak plain HTTP only and the installer carries no signature. So `install.rs` keeps the SHA-1 of the
`install.exe` it was tried on (`KNOWN_INSTALLERS`: `7a2ba755…f3c9c0`, 18.8.50_EP1) and refuses another unless
`--allow-unknown-installer` is given. That one hash covers the slices too: `install.exe` holds the SHA-1 of every
file in the package, and each is checked as it is unpacked.

## 2. The installer is an Inno Setup 5.4.2 package

`install.exe` and its five slices are an **Inno Setup 5.4.2 (a)** package (the ANSI build) of version 18.8.50_EP1,
built in June 2020. `inno.rs` reads it; the layout was learned from the open-source `innoextract`
(<https://constexpr.org/innoextract/>), and the module's own text describes it:

- `install.exe` has an offset table in its resources (`RCDATA` 11111, `rDlPtS…`) that says where the setup data starts
  (0xd400): a version line, then two CRC-checked blocks of LZMA1, the first with the header and every entry list, the
  second with one data entry for each packed file (its slice, offset, size, SHA-1, time, flags).
- The slices (`idska32\x1a`) hold the files in chunks (`zlb\x1a`). This package is one chunk: a single LZMA1 stream
  (8 MB dictionary) of 2,096,905,388 bytes that starts in the first slice and runs through all five, with the 7,098
  files one after the other in it (3,133,054,176 bytes unpacked). So no file can be reached without unpacking
  everything before it. 54 program files are stored with their x86 call targets made absolute, which unpacking
  undoes. Nothing is encrypted.
- The package has no licence page: the header's `license_text` and `info_before` are empty. Funcom's licence is
  shown by the downloader before it fetches the package, and by the game's launcher before the first start
  (`anarchy.eula`). `install` shows the package's own `anarchy.eula` and asks (or takes `--accept-eula`) before it
  unpacks anything else: the file lies 1.3 MB into the stream, so it is reached at once (`Installer::licence`).
- Every file goes to `{app}`, without conditions. A file is written under another name and renamed when its SHA-1 is
  right; its time is the package's (the packager's local time, taken as UTC). Unpacking takes about two and a half
  minutes.

What the installer does besides copying files makes nothing in the game folder: a shortcut in the start menu (and on
the desktop when asked), Inno's own uninstaller (`unins000.exe`, `unins000.dat`, an uninstall key in the registry)
and an offer to start the game. It has no `[Code]`, `[INI]`, `[Registry]`, `[Dirs]` or `[InstallDelete]` entries.
`anarchy.cfg`, `version.id`, `Setupf` and `locales` are plain files of the package. So the unpacked folder is the
installed game. Only version 5.4.2 (ANSI) with LZMA1 or stored chunks is read; anything else is refused.

## 3. The patcher

The installer leaves 18.8.50_EP1; the servers run 18.8.62. `Anarchy.exe` (the launcher) and `AnarchyPatcher.exe` close
the gap, and `patcher.rs` does the same:

1. **Local version**: the first line of `version.id` (`18.8.50_EP1`).
2. **Remote version**: `cd_image/data/launcher/DimensionServer.url` names
   `dimensions.anarchy-online.com:80/new-dimensions/dimensions_v3.txt`; each dimension there has `version = 18.8.62`.
   The game's own ending (`_EP1`) goes onto it.
3. **Patch servers**: the same folder's `patchservers.txt` (87 bytes):
   `Live;1;http://update.anarchy-online.com:80/c` and `Live;5;http://cdn-web-nj4.funcom.com:80/c`.
4. **The map**: `<server>/map` (11,580 bytes, 580 lines), each line `<from> <to>`, e.g. `18.8.50_EP1 18.8.51_EP1`.
   The patcher walks it once from the local version (`FUN_0045b1c0`), naming each step `v<from>-v<to>`. From
   18.8.50_EP1 there are **12 steps**, one version at a time, to 18.8.62_EP1.
5. **A step's files** (`FUN_0045d0e0`, the format string `%s/%s/%s.%s` pushed at `0x45dc90`):
   `<server>/<step>/<STEP>.<EXT>`, where `<STEP>` is the step's name with `.` turned into `-` and in capitals
   (`_mbsupr_s` at 0x5c291b, which Ghidra names `_mbslwr_s`), and `<EXT>` is `RTP`, `RES`, `CHK` and `SIG` (the table
   at `0x66d438`):
   `http://update.anarchy-online.com/c/v18.8.50_EP1-v18.8.51_EP1/V18-8-50_EP1-V18-8-51_EP1.RTP`.
   They are kept in the game's `Patches` folder while the step is applied, as the original keeps them, and files
   already there are used.
6. **The checks** (`crypto.rs`): the `.CHK` is text, `<file name>:<MD5 in hex>` for the step's `.RTP` and `.RES`. The
   `.SIG` is the signature of the `.CHK`, in hex, for the key in `cd_image/data/launcher/funcom.pub` (hex of an X.509
   key, RSA 768 bits): RSASSA-PKCS1-v1_5 with SHA-1 over the file's bytes. `FUN_0046a800` is Crypto++'s
   `RSAVerifyFile` example. A step whose files do not pass is not applied, and its files are deleted.
7. **Applying**: the `.RES` first, then the `.RTP` (the original's order; the `.RTP` is what moves `version.id` on, so
   a step cut short starts again and finds its database records already in).
8. **`rename.ini`**: a step leaves `AnarchyPatcher.exe=ap.exe` behind for the launcher, which moves the new patcher
   over the old one ("Performing patch cleanup...") and deletes the file. Done after each step.

The 12 steps on the server (bytes):

| Step | `.RTP` | `.RES` | records in the `.RES` |
|---|---|---|---|
| 18.8.50 → 51 | 1,944,299 | 16 | 0 |
| 51 → 52 | 1,519,627 | 5,621 | 7 |
| 52 → 53 | 1,519,850 | 9,648 | 25 |
| 53 → 54 | 1,709,095 | 1,016,358 | 430 |
| 54 → 55 | 1,517,609 | 11,212 | 43 |
| 55 → 56 | 1,523,386 | 16 | 0 |
| 56 → 57 | 1,517,273 | 16 | 0 |
| 57 → 58 | 1,633,711 | 6,616,205 | 272 |
| 58 → 59 | 1,519,092 | 1,578 | 5 |
| 59 → 60 | 1,519,582 | 16 | 0 |
| 60 → 61 | 1,518,967 | 20,917 | 5 |
| 61 → 62 | 1,540,369 | 236 | 1 |

With the `.CHK` (321 bytes) and `.SIG` (196) files, 26,670,903 bytes in all.

After patching the original launcher also compares the programs with `cd_image/data/launcher/checksums.chk` (MD5s,
signed the same way) and can fetch a whole file from `<server>/Maintenance/…` when one is wrong. Not done here: the
client reads none of the programs.

## 4. The `.RTP` files: RTPatch 5.21

A `.RTP` is a patch file of **Pocket Soft RTPatch 5.21**, which the patcher hands to `PATCHW32.DLL`
(`RTPatchApply32@12`). `rtpatch.rs` reads and applies it; there is no public description of the format, so all of this
is from the DLL.

**The file** (`FUN_1000b420`, `FUN_1000acd0`): `u16` 0x2a4b, `u16` version 0x209, `u16` flags, more flags, the default
flags of entries, disk space wanted, the backup folder's name (`PATCHESUNDO`), and a list of 506 folder names. Numbers
are mostly variable-length (`FUN_10019260`): the first byte's top bit the sign, then one bit for each byte that
follows, the rest of the first byte the number's top part. A string is a length byte and the characters with a NUL.

**Entries** (`FUN_10013c70`, `FUN_10008f10`) until one of type 1; the type is the top four bits of the entry's flags:

| Type | | Handler |
|---|---|---|
| 2 | a new file: the data is the file | `0x10001000` |
| 3 | a file to delete | `0x10004c70` |
| 4 | a file to change: the data is a list of commands | `0x10002820` |
| 5, 6 | (history and rename: not in Funcom's patches, refused) | `0x10007420`, `0x100135c0` |

An entry names its folder by number, and each of its old and new files by an 8.3 name, attributes, size, DOS time
and date, a ten-byte checksum and the long name. Funcom's steps each have 32 entries of type 4 (the game's programs)
and 11 or 12 of type 2 (`version.id`, `mapid.txt`, the checksum list and its signature, the new patcher as `ap.exe`,
`rename.ini`, a few `.url` files), and now and then a playfield file (`cd_image/data/statels/111.pf`, `9070.pf`,
`9080.pf`, `cd_image/data/shadows/111.sdw`).

**The checksum** (`FUN_10019bd0`): the length modulo 31 and modulo 30, and two words, 31 and 30 bits wide, that each
byte is exclusive-ored into while they turn left by eight.

**The packing** of an entry's data (`FUN_1001b430`, `FUN_1001ac10`, `FUN_1001af00`): 16 bits 0xb59c, a few settings,
then bytes and copies of earlier bytes, each led by a bit that says which. Three prefix codes are used, for bytes,
copy lengths and the high bits of copy distances, and all three change as the data goes: a code starts with one escape
symbol; a symbol met for the first time is the escape and then its own bits; every so many symbols the symbols are
sorted by how often they came and the code lengths moved towards what suits (`FUN_1001bf90`, `FUN_1001ad30`,
`FUN_1001bea0`, `FUN_1001bb10`). Exactness matters down to the order a sort leaves equal symbols in, so the code
follows the DLL's steps.

**The commands** of a changed file (`FUN_10014790`): copy a range of the old file; leave a hole; fill with a one-,
two- or four-byte pattern; add a value to bytes, words or double words already written (how a relinked program's
addresses are moved); and command 5, which brings the bytes of all the holes together.

**What `rtpatch::apply` does**: every old file is checked against the patch's checksum for it, every new file is made
in memory and checked against its own, and only then are the files written (each beside its place first, then moved
over it). A file that is already what the patch makes of it is left alone. RTPatch's backup of the old files
(`PatchesUndo`, with a script that puts them back) is not kept.

## 5. The `.RES` files: Funcom's database patch

`AnarchyPatcher.exe` `FUN_004662f0`; `respatch.rs`. Big-endian words: 0xf1576, 0xf1577 or 0xf1578; a count for the
progress bar; the records to delete (type, instance); the records to add or replace. A record comes whole, packed with
zlib; or, for types 0xf4241, 0xf4249, 0xf424a and 0xf425a since 0xf1577, as an RTPatch file for one file called
`tmp.dat`: the patcher writes the record now in the database to `tmp.dat` (no file when the record is new), runs
RTPatch on it and takes the new record back. An empty patch is 16 bytes. The twelve steps bring 788 records, 12 of
them as patches, and delete none.

The database is c-tree Plus files (`ResourceDatabase.dat`, `.dat.001`, `.dat.002`, `.idx`: 2.3 GB as installed). A
record is rewritten where it is when the new one fits and added at the end when not, and its key added to or moved in
the index's B-tree, as the engine does it (`FUN_0046c250`). The writing engine is `ctree/`, brought over from
`AnarchyReimagined`'s `ao-ctree`, which was written from the file format and tested there against the original's
`DatabaseController.dll`; the client's own reader in `mao-formats` only reads.

**One fault found in Funcom's own database**: the last key of the index the installer brings (0xfde97:305696) has a
high position word of 1 though its record is where the low word says. The original engine never reads the high word.
The client's reader does (it has to, for a database grown past 4 GB), so it now falls back to the low word alone when
the full position holds no record of the key (`mao-formats` `ctree.rs`, `Database::record`).

## 6. What was checked, and how to check it again

The tests that need Funcom's files read their places from the environment and pass without doing anything when a
place is not given:

| Variable | What it is |
|---|---|
| `MAO_INSTALLER` | the folder with `install.exe` and its slices |
| `MAO_PATCHES` | the folder with the twelve steps' files |
| `MAO_GAME` | an installed game that the original patcher brought to 18.8.62, with its `PatchesUndo` folder |
| `MAO_FRESH_GAME` | a game folder as the installer leaves it |
| `MAO_SCRATCH` | room for a copy of the database (2.3 GB) |
| `MAO_MODDED_TYPES` | record types that the compared game's owner changed (`f6954` for a texture pack) |
| `MAO_INNO_OUT` | a folder to unpack the whole package into (3 GB, two and a half minutes) |
| `MAO_PATCHED_GAME` | a game folder that `install` or `patch` made, to compare with `MAO_GAME` |

- **RTPatch, every step** (`tests/patches.rs`): `PatchesUndo` of an installed game still holds the 36 loose files of
  every version from 18.8.50 to 18.8.61. Each step's `.RTP` is applied to the files of its version: all 43 or more
  entries pass RTPatch's own checksums, and the result is byte for byte the kept files of the next version (11 steps;
  the twelfth has no kept next version and passes by the checksums). Applied a second time, nothing changes. A step
  applied to another version's files is refused with nothing written.
- **Signatures** (same file): Funcom's signature of each step's checksum list passes for the game's `funcom.pub`, and
  the lists' MD5s match the patch files; a list or a patch with a byte changed does not pass. The game's own
  `checksums.chk` passes too.
- **The database** (`tests/database.rs`): the twelve `.RES` files applied to a copy of the installer's database, then
  every one of the 460,193 records compared with an installed game's through the client's own reader. All agree,
  the 12 that come as RTPatch patches among them, except records of type 0xf6954 (textures) where the compared game
  had a texture pack put in: 12,581 of those differ, 28 of them ones the patches bring, so those 28 are checked only
  by their zlib sizes. The patched database has exactly the keys of the original's. Each step applied a second time
  changes nothing.
- **The package** (`tests/inno.rs`; the whole unpacking needs `MAO_INNO_OUT`, a folder to unpack into): all 7,098
  files come out with the SHA-1 the package gives them. Against an installed, patched game, by path without regard to
  case: 7,054 files identical, none missing, 44 different. Those 44 are what the twelve patch steps change (33
  programs, `version.id`, `mapid.txt`, the database's four files, the checksum list and its signature, `111.pf`,
  `111.sdw`) and `cd_image/gui/Default/OptionPanel/Root.xml`, which is empty in the installed game compared.
- **The whole way** (2026-11-25, on Windows): `mao-client install <new folder> --accept-eula`, the patches fetched
  from Funcom's server as it ran. The folder it made says 18.8.62_EP1 and:
  - all 47 files that Funcom's signed `checksums.chk` lists (the programs, `text.mdb`, the GUI archives) have the
    MD5 the list gives;
  - of its 7,100 files, 7,095 are byte for byte those of a game the original installer and patcher made; the other
    five are the database's four files (compared by record, next) and `OptionPanel/Root.xml` (empty in the game
    compared). No file is missing from it that the original's has, bar what Windows add-ons, the original patcher's
    backups and Inno's uninstaller left there;
  - its database has the original's 460,193 keys, and every record the same but the texture pack's
    (`a_patched_game_has_the_originals_records`, with `MAO_PATCHED_GAME`);
  - the client's own game-data tests (`mao-formats` `tests/game_data.rs`) pass with `MAO_GAME` set to it.
  `patch` on that folder says there is nothing to patch; `install` on it again does nothing but ask the servers;
  `install` into a folder that holds other things refuses.

## 7. In the launcher

`launchinstall.rs` puts both into the launcher's window (`mao-client`, `mao-client launch`). The original's launcher
has neither: it is part of the installed game, and it starts `AnarchyPatcher.exe`.

- **No game folder.** The launcher opens on a window that says no Anarchy Online folder was found and offers the two
  ways on: *Choose its folder* (the settings' GUI page, which is what the launcher used to open on) or *Download and
  install* into a folder. The folder is Funcom's own default on Windows (`C:\Funcom\Anarchy Online`) and
  `~/Games/Anarchy Online` elsewhere, which the launcher also looks through by itself from now on; Browse names
  another. Under the field is what the folder is: free, not empty (refused), an install that was cut short (it goes
  on), or the game already (the button is then *Use this folder*). PLAY's place holds INSTALL, which opens the
  window again after it was closed. The licence (`anarchy.eula` from the package, section 2) is shown before anything
  is unpacked; having accepted it there, the player is not asked for the same text again by the launcher's own
  licence window. The folder is written to the settings when the install begins, so a later start offers it again;
  when the game is in, the launcher starts again with it, as it does after another folder was chosen in the settings.
- **PATCH.** PATCH patches the game in a window of the launcher's, and PLAY comes when the patch is in: on every
  system, Windows too (the user's choice, 2026-11-25). Windows is the one system that can run Funcom's
  `AnarchyPatcher.exe`, and there the settings' Connection page has "Patch with Funcom's patcher"
  (`patcher = funcom` in `launcher.ini`), which starts it as the original's launcher does. When the client's own
  patcher fails, its window says why and offers *Try again* and, on Windows, *Use Funcom's patcher*: a game under
  Windows' program folders cannot be written to without an administrator's rights, which Funcom's patcher asks for
  and the client's does not.
- **Stopping.** The work runs off the window's thread and can be stopped at any time, by its Stop button or by closing
  the launcher: a download keeps what it has (`.part`), the unpacking stops at the file it is at (the folder keeps
  its `mao-install.unpacking` mark and is unpacked anew), and the patching stops before a step's files are touched.
  A step that has begun to write is finished (`patcher::Event::Writing`; the resource database must not be left half
  written): a launcher closed in those seconds stays until the step is in, then goes.

Checked (2026-11-25, on Windows, the launcher's windows photographed from the windows themselves, each run with its
settings in a scratch folder and `MAO_NO_USUAL_DIRS=1` where there was to be no game):

- with no game: the window and INSTALL; a folder that has the game, one that is free, one whose install was cut
  short are each told as what they are;
- an install through the launcher into a new folder, the installer's parts already downloaded
  (`MAO_INSTALL_DOWNLOADS`): the licence shown, then the unpacking and the twelve patch steps; the launcher started
  again by itself with PLAY and did not ask for the licence a second time. The folder says 18.8.62_EP1 and all 47
  files of Funcom's signed `checksums.chk` have its MD5s (twice: once with the patches fetched from Funcom's server
  as it ran, once with them put into the folder's `Patches` beforehand);
- Stop pressed in the unpacking: the window goes, the foot says it stopped, the folder keeps its mark and no
  half-written file, and the next start offers the folder as an install cut short;
- PATCH with the client's own patcher on a game as the installer leaves it (18.8.50), the launcher closed in the
  middle: it went at once and left the game whole at 18.8.56, six steps in. PATCH again took it to 18.8.62 and
  PATCH became PLAY. That folder has the 47 listed files right, and every one of its database's 460,193 records is
  the installed game's (the texture pack's apart, as in section 6);
- the same PATCH with a `launcher.ini` that names no patcher (the default): 18.8.50 to 18.8.62, the 47 listed files
  right; the Connection page shows the box unticked;
- an install that cannot write its folder: the window says so and offers another try.

Not checked: Funcom's patcher started from the failure window's button or by the Connection page's box (the code it
runs is the one PATCH ran before, moved into `start_funcom_patcher`); a patch of a game under the program folders;
the launcher's windows on Linux or macOS; a download cut short and gone on with through the launcher (the same
`fetch::file` as the command's).

For pictures and unattended runs: `MAO_LAUNCH_INSTALL=<folder>` (with no game: the install into that folder starts
by itself), `MAO_LAUNCH_ACCEPT=<seconds>` (the licence is accepted that long after it is shown),
`MAO_LAUNCH_PATCH=<seconds>` (PATCH presses itself), `MAO_LAUNCH_STOP=<seconds>` (Stop presses itself that long into
the work), `MAO_INSTALL_DOWNLOADS=<folder>` (where the installer's parts are; they are kept).

## 8. What is not done

- The client's own patcher does not ask Windows for an administrator's rights: a game in a folder that needs them
  (the program folders) fails with the system's "access denied", and the failure window's *Use Funcom's patcher* is
  the way on.
- RTPatch: entry types 5 and 6, entries with more than one file, parts of a file left out of the checksum, passwords,
  registry parts. Funcom's patches use none; a patch that does is refused, not guessed at. Type 3 (delete) is written
  but no patch of Funcom's has one, so it has never met real data.
- `.RES`: the record 0xf4254:0 (`FUN_0046f0e0`), and deletes, which are written but which no patch of these twelve
  has.
- A database left half-written by a crash in the middle of a step: the engine marks the files as being changed, and
  both it and the client's reader then refuse them. The original patcher rebuilds the index in that case; here the
  folder has to be unpacked again.
- The original patcher's `Maintenance` repair of single files, and its backup of old files in `PatchesUndo`.
- Linux and macOS: the code has no system-specific part and matches file names without regard to case, but it has
  only been run on Windows.
