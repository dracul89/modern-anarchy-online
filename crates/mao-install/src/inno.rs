//! Reading an Inno Setup 5.4.2 package (`install.exe` and its `install-N.bin` slices).
//!
//! Funcom's installer for Anarchy Online is an Inno Setup 5.4.2 (ANSI) package: a small `install.exe` that holds
//! the description of the install (the "setup data": where each file goes, registry entries, shortcuts and so on)
//! and five `install-N.bin` slices that hold the files themselves, packed with LZMA. This module reads that one
//! format version so the game's files can be unpacked on a machine that cannot run a Windows installer. The
//! installer is treated as data only: nothing in it is ever run.
//!
//! The layout of the setup data, of the slices and of the call-instruction filter was learned from the open-source
//! **innoextract** project by Daniel Scharrer (<https://constexpr.org/innoextract/>, zlib licence; its
//! `src/setup/*.cpp`, `src/stream/*.cpp` and `src/loader/offsets.cpp`), which documents every Inno Setup version.
//! The code here is our own and covers only what version 5.4.2 (ANSI) uses.
//!
//! # The format in short
//!
//! * `install.exe` is a Windows program with an *offset table* in its resources (`RCDATA` 11111, magic
//!   `rDlPtS…`), checked by a CRC-32. It says where the setup data starts in the file.
//! * The setup data starts with a 64-byte version line (`Inno Setup Setup Data (5.4.2)`), followed by two
//!   *blocks*. A block is a CRC-checked 9-byte head (stored size, packed or not) and its bytes in pieces of 4096,
//!   each piece led by its CRC-32; put together they are one LZMA1 stream. The first block holds the header and
//!   every entry list (languages, messages, types, components, tasks, directories, files, icons, ini entries,
//!   registry entries, things to delete, programs to run); the second holds one *data entry* per packed file.
//! * A file entry names its data entry by index. A data entry says in which slice and at which offset its *chunk*
//!   starts (`zlb\x1a`, then the packed bytes, which may run on into the next slices), where the file lies in the
//!   unpacked chunk, its size, SHA-1, time and flags. Several files can share one chunk ("solid" packing).
//! * Program files may be stored with their x86 `call`/`jmp` targets made absolute so they pack better; unpacking
//!   turns them back ([`CallFilter`]).
//!
//! # What an installer does besides copying files
//!
//! Unpacking gives the files only. The other entry lists are parsed and can be read from [`Package`]
//! (`directories`, `ini_entries`, `registry_entries`, `run_entries`, `install_delete`, `icons`, …) so a caller can
//! see what else an installer would have done; nothing here carries those out.
//!
//! For Funcom's 18.8.50 package (read with this module, and borne out by the uninstall log of a real install) the
//! answer is "nothing the game needs":
//!
//! * `[Files]`: 7098 packed files, every one below `{app}`, with no component, task, language or script condition
//!   and the flags `ignoreversion createallsubdirs`; 3,133,054,176 bytes in one solid LZMA1 chunk (8 MB
//!   dictionary) of 2,096,905,388 bytes across the five slices. 54 program files use the call filter. Nothing is
//!   encrypted. `anarchy.cfg`, `version.id`, `Setupf\` and `locales\` are ordinary packed files, not written by
//!   any other step. One more entry stands for the uninstaller.
//! * No `[Dirs]`, `[INI]`, `[Registry]`, `[InstallDelete]`, `[UninstallDelete]`, `[UninstallRun]`, `[Types]` or
//!   `[Components]` entries, no `[Code]` script, no password, no licence page. Folders exist only because files
//!   go into them.
//! * `[Tasks]`: `desktopicon`, not ticked by default. `[Icons]`: a start-menu shortcut `{group}\Anarchy Online`
//!   and, with that task, a desktop one, both to `{app}\Anarchy.exe`.
//! * `[Run]`: `{app}\Anarchy.exe` as the "launch now" box on the last page (`postinstall nowait skipifsilent`).
//! * Inno Setup itself adds `unins000.exe` and `unins000.dat` to `{app}` and its uninstall key under
//!   `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall`. The default folder is
//!   `{sd}\Funcom\Anarchy Online` and the installer asks for administrator rights.
//!
//! # Not handled
//!
//! Other Inno Setup versions and Unicode builds; chunks packed with zlib, bzip2 or LZMA2; encrypted packages;
//! packages whose data is inside the installer program instead of slice files; files the installer would take
//! from outside the package. These are refused with [`Error::Unsupported`] or, for the last, left out of a plan.
//! Text is read as Windows code page 1252. `Check:` conditions and `[Code]` scripts are never evaluated, Windows
//! version limits and file attributes are not applied, and a file time stored as the packager's local time (as in
//! Funcom's package) is taken as UTC, since the package does not say which time zone it was. The readers for the
//! entry kinds Funcom's package has none of (`[Dirs]`, `[INI]`, `[Registry]`, `[InstallDelete]`, `[Types]`,
//! `[Components]`) follow innoextract's description but have not met real data.

mod lzma;

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::sha1::Sha1;
use lzma::Lzma1Reader;

/// The only setup data version read here.
const VERSION_LINE: &str = "Inno Setup Setup Data (5.4.2)";
const OFFSET_TABLE_MAGIC: [u8; 12] = [b'r', b'D', b'l', b'P', b't', b'S', 0xcd, 0xe6, 0xd7, b'{', 0x0b, b'*'];
const SLICE_MAGIC: &[u8; 8] = b"idska32\x1a";
const CHUNK_MAGIC: &[u8; 4] = b"zlb\x1a";
/// 1 January 1970 as a Windows FILETIME (100 ns steps since 1601).
const FILETIME_UNIX_EPOCH: i64 = 0x019d_b1de_d53e_8000;
/// How much of a file is unpacked, checked and written at a time.
const COPY_BUFFER: usize = 1 << 18;

// ---------------------------------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------------------------------

/// Why a package could not be read or unpacked.
#[derive(Debug)]
pub enum Error {
    /// Reading or writing a file failed.
    Io(io::Error),
    /// The bytes are not what the format says they should be.
    Format(String),
    /// The package uses something this reader does not handle (another Inno Setup version, another compression
    /// method, encryption): refused rather than guessed at.
    Unsupported(String),
    /// A check sum does not match: the download is damaged or incomplete.
    Checksum { what: String, expected: String, actual: String },
    /// A file of the package (`install.exe` or a slice `install-N.bin`) is missing from the folder.
    MissingFile(String),
    /// The caller's progress function said to stop: the file being written is taken away again.
    Stopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Format(s) => write!(f, "not a readable Inno Setup package: {s}"),
            Error::Unsupported(s) => write!(f, "not supported: {s}"),
            Error::Checksum { what, expected, actual } => {
                write!(f, "check sum of {what} is {actual}, should be {expected}")
            }
            Error::MissingFile(s) => write!(f, "missing package file {s}"),
            Error::Stopped => write!(f, "stopped"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    /// (an error of ours that travelled through a `Read` comes back out as itself)
    fn from(e: io::Error) -> Self {
        match e.downcast::<Error>() {
            Ok(inner) => inner,
            Err(e) => Error::Io(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn format_err<T>(text: impl Into<String>) -> Result<T> {
    Err(Error::Format(text.into()))
}

// ---------------------------------------------------------------------------------------------------------------
// Flag sets
// ---------------------------------------------------------------------------------------------------------------

/// Defines the bits of one stored flag set, in the order Inno Setup 5.4.2 stores them, and a `names` function for
/// showing them.
macro_rules! flag_set {
    ($(#[$meta:meta])* $module:ident: $ty:ty { $($name:ident),* $(,)? }) => {
        $(#[$meta])*
        pub mod $module {
            flag_set!(@bits $ty, 0u32; $($name,)*);
            /// The flags' names, lowest bit first.
            pub const NAMES: &[&str] = &[$(stringify!($name)),*];
            /// How many bits the stored set has.
            pub const COUNT: u32 = NAMES.len() as u32;
            /// The names of the flags set in `flags`.
            pub fn names(flags: $ty) -> Vec<&'static str> {
                NAMES.iter().enumerate().filter(|(i, _)| flags >> i & 1 != 0).map(|(_, n)| *n).collect()
            }
        }
    };
    (@bits $ty:ty, $n:expr; ) => {};
    (@bits $ty:ty, $n:expr; $name:ident, $($rest:ident,)*) => {
        pub const $name: $ty = 1 << ($n);
        flag_set!(@bits $ty, $n + 1u32; $($rest,)*);
    };
}

flag_set! {
    /// `[Setup]` switches of the header.
    header_flags: u64 {
        DISABLE_STARTUP_PROMPT, CREATE_APP_DIR, ALLOW_NO_ICONS, ALWAYS_RESTART, ALWAYS_USE_PERSONAL_GROUP,
        WINDOW_VISIBLE, WINDOW_SHOW_CAPTION, WINDOW_RESIZABLE, WINDOW_START_MAXIMIZED,
        ENABLE_DIR_DOESNT_EXIST_WARNING, PASSWORD, ALLOW_ROOT_DIRECTORY, DISABLE_FINISHED_PAGE, CHANGES_ASSOCIATIONS,
        USE_PREVIOUS_APP_DIR, BACK_COLOR_HORIZONTAL, USE_PREVIOUS_GROUP, UPDATE_UNINSTALL_LOG_APP_NAME,
        USE_PREVIOUS_SETUP_TYPE, DISABLE_READY_MEMO, ALWAYS_SHOW_COMPONENTS_LIST, FLAT_COMPONENTS_LIST,
        SHOW_COMPONENT_SIZES, USE_PREVIOUS_TASKS, DISABLE_READY_PAGE, ALWAYS_SHOW_DIR_ON_READY_PAGE,
        ALWAYS_SHOW_GROUP_ON_READY_PAGE, ALLOW_UNC_PATH, USER_INFO_PAGE, USE_PREVIOUS_USER_INFO,
        UNINSTALL_RESTART_COMPUTER, RESTART_IF_NEEDED_BY_RUN, SHOW_TASKS_TREE_LINES, ALLOW_CANCEL_DURING_INSTALL,
        WIZARD_IMAGE_STRETCH, APPEND_DEFAULT_DIR_NAME, APPEND_DEFAULT_GROUP_NAME, ENCRYPTION_USED,
        CHANGES_ENVIRONMENT, SHOW_UNDISPLAYABLE_LANGUAGES, SETUP_LOGGING, SIGNED_UNINSTALLER, USE_PREVIOUS_LANGUAGE,
        DISABLE_WELCOME_PAGE,
    }
}

flag_set! {
    /// `[Files]` flags.
    file_flags: u32 {
        CONFIRM_OVERWRITE, NEVER_UNINSTALL, RESTART_REPLACE, DELETE_AFTER_INSTALL, REGISTER_SERVER,
        REGISTER_TYPE_LIB, SHARED_FILE, COMPARE_TIME_STAMP, FONT_IS_NOT_TRUE_TYPE, SKIP_IF_SOURCE_DOESNT_EXIST,
        OVERWRITE_READ_ONLY, OVERWRITE_SAME_VERSION, CUSTOM_DEST_NAME, ONLY_IF_DEST_FILE_EXISTS, NO_REG_ERROR,
        UNINS_RESTART_DELETE, ONLY_IF_DOESNT_EXIST, IGNORE_VERSION, PROMPT_IF_OLDER, DONT_COPY,
        UNINS_REMOVE_READ_ONLY, RECURSE_SUB_DIRS_EXTERNAL, REPLACE_SAME_VERSION_IF_CONTENTS_DIFFER,
        DONT_VERIFY_CHECKSUM, UNINS_NO_SHARED_FILE_PROMPT, CREATE_ALL_SUB_DIRS, BITS_32, BITS_64,
        EXTERNAL_SIZE_PRESET, SET_NTFS_COMPRESSION, UNSET_NTFS_COMPRESSION, GAC_INSTALL,
    }
}

flag_set! {
    /// Flags of a data entry (one packed file).
    data_flags: u16 {
        VERSION_INFO_VALID, VERSION_INFO_NOT_VALID, TIME_STAMP_IN_UTC, IS_UNINSTALLER_EXE,
        CALL_INSTRUCTION_OPTIMIZED, TOUCH, CHUNK_ENCRYPTED, CHUNK_COMPRESSED, SOLID_BREAK,
    }
}

flag_set! {
    /// `[Dirs]` flags.
    directory_flags: u8 {
        NEVER_UNINSTALL, DELETE_AFTER_INSTALL, ALWAYS_UNINSTALL, SET_NTFS_COMPRESSION, UNSET_NTFS_COMPRESSION,
    }
}

flag_set! {
    /// `[Icons]` flags.
    icon_flags: u8 {
        NEVER_UNINSTALL, CREATE_ONLY_IF_FILE_EXISTS, USE_APP_PATHS, FOLDER_SHORTCUT, EXCLUDE_FROM_SHOW_IN_NEW_INSTALL,
    }
}

flag_set! {
    /// `[INI]` flags.
    ini_flags: u8 {
        CREATE_KEY_IF_DOESNT_EXIST, UNINS_DELETE_ENTRY, UNINS_DELETE_ENTIRE_SECTION, UNINS_DELETE_SECTION_IF_EMPTY,
        HAS_VALUE,
    }
}

flag_set! {
    /// `[Registry]` flags.
    registry_flags: u16 {
        CREATE_VALUE_IF_DOESNT_EXIST, UNINS_DELETE_VALUE, UNINS_CLEAR_VALUE, UNINS_DELETE_ENTIRE_KEY,
        UNINS_DELETE_ENTIRE_KEY_IF_EMPTY, PRESERVE_STRING_TYPE, DELETE_KEY, DELETE_VALUE, NO_ERROR, DONT_CREATE_KEY,
        BITS_32, BITS_64,
    }
}

flag_set! {
    /// `[Run]` and `[UninstallRun]` flags.
    run_flags: u16 {
        SHELL_EXEC, SKIP_IF_DOESNT_EXIST, POST_INSTALL, UNCHECKED, SKIP_IF_SILENT, SKIP_IF_NOT_SILENT, HIDE_WIZARD,
        BITS_32, BITS_64, RUN_AS_ORIGINAL_USER,
    }
}

flag_set! {
    /// `[Components]` flags.
    component_flags: u8 { FIXED, RESTART, DISABLE_NO_UNINSTALL_WARNING, EXCLUSIVE, DONT_INHERIT_CHECK }
}

flag_set! {
    /// `[Tasks]` flags.
    task_flags: u8 { EXCLUSIVE, UNCHECKED, RESTART, CHECKED_ONCE, DONT_INHERIT_CHECK }
}

// ---------------------------------------------------------------------------------------------------------------
// What the setup data holds
// ---------------------------------------------------------------------------------------------------------------

/// How the files' chunks are packed (`Compression=` of the script).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    Stored,
    Zlib,
    BZip2,
    Lzma1,
    Lzma2,
}

/// A Windows version as Inno Setup stores it for `MinVersion`/`OnlyBelowVersion`: one for Windows 9x, one for NT.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsVersion {
    /// `(major, minor, build)` for the Windows 95 family.
    pub win: (u8, u8, u16),
    /// `(major, minor, build)` for the Windows NT family.
    pub nt: (u8, u8, u16),
    /// `(major, minor)` of the NT service pack.
    pub service_pack: (u8, u8),
}

/// The conditions any entry can carry. An empty string means "no condition".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conditions {
    /// `Components:` expression (names joined by `and`/`or`/`not`).
    pub components: String,
    /// `Tasks:` expression.
    pub tasks: String,
    /// `Languages:` expression.
    pub languages: String,
    /// `Check:` — a call into the installer's Pascal script; it cannot be evaluated here.
    pub check: String,
    /// `AfterInstall:` script call.
    pub after_install: String,
    /// `BeforeInstall:` script call.
    pub before_install: String,
    /// `MinVersion:`.
    pub min_version: WindowsVersion,
    /// `OnlyBelowVersion:` (all zero: no limit).
    pub only_below_version: WindowsVersion,
}

impl Conditions {
    /// No component, task, language or script condition (Windows version limits are not counted).
    pub fn is_unconditional(&self) -> bool {
        self.components.is_empty() && self.tasks.is_empty() && self.languages.is_empty() && self.check.is_empty()
    }
}

/// The `[Setup]` section.
#[derive(Debug, Clone, Default)]
pub struct Header {
    pub app_name: String,
    pub app_versioned_name: String,
    pub app_id: String,
    pub app_copyright: String,
    pub app_publisher: String,
    pub app_publisher_url: String,
    pub app_support_phone: String,
    pub app_support_url: String,
    pub app_updates_url: String,
    pub app_version: String,
    /// `DefaultDirName`: where `{app}` is unless the user picks another folder.
    pub default_dir_name: String,
    pub default_group_name: String,
    pub base_filename: String,
    pub uninstall_files_dir: String,
    pub uninstall_name: String,
    pub uninstall_icon: String,
    pub app_mutex: String,
    pub default_user_name: String,
    pub default_user_organisation: String,
    pub default_serial: String,
    pub app_readme_file: String,
    pub app_contact: String,
    pub app_comments: String,
    pub app_modify_path: String,
    pub create_uninstall_registry_key: String,
    pub uninstallable: String,
    pub license_text: String,
    pub info_before: String,
    pub info_after: String,
    /// The installer's compiled Pascal script (`[Code]`), if any. Never run; kept so a caller can see that there is
    /// one and how big it is.
    pub compiled_code: Vec<u8>,
    pub min_version: WindowsVersion,
    pub only_below_version: WindowsVersion,
    /// SHA-1 of the install password with its salt (meaningful only with the `PASSWORD` flag).
    pub password_hash: [u8; 20],
    pub password_salt: [u8; 8],
    pub extra_disk_space_required: i64,
    pub slices_per_disk: u32,
    /// 0 none, 1 power user, 2 admin, 3 lowest.
    pub privileges_required: u8,
    /// `None` for a method number this version does not define.
    pub compression: Option<Compression>,
    pub uninstall_display_size: u32,
    /// See [`header_flags`].
    pub flags: u64,
}

/// One `[Languages]` entry.
#[derive(Debug, Clone, Default)]
pub struct Language {
    pub name: String,
    pub language_name: String,
    pub language_id: u32,
    pub codepage: u32,
}

/// One `[CustomMessages]` (or overridden message) entry.
#[derive(Debug, Clone, Default)]
pub struct Message {
    pub name: String,
    pub value: String,
    /// Index into the languages, or -1 for all.
    pub language: i32,
}

/// One `[Types]` entry (full/compact/custom installs).
#[derive(Debug, Clone, Default)]
pub struct SetupType {
    pub name: String,
    pub description: String,
    pub languages: String,
    pub check: String,
    pub custom: bool,
    pub size: u64,
}

/// One `[Components]` entry.
#[derive(Debug, Clone, Default)]
pub struct Component {
    pub name: String,
    pub description: String,
    /// The setup types that include it, separated by spaces.
    pub types: String,
    pub languages: String,
    pub check: String,
    pub extra_disk_space_required: u64,
    pub level: i32,
    pub used: bool,
    /// See [`component_flags`].
    pub flags: u8,
    pub size: u64,
}

/// One `[Tasks]` entry (a check box on the "additional tasks" page).
#[derive(Debug, Clone, Default)]
pub struct Task {
    pub name: String,
    pub description: String,
    pub group_description: String,
    pub components: String,
    pub languages: String,
    pub check: String,
    pub level: i32,
    pub used: bool,
    /// See [`task_flags`]. A task is ticked by default unless `UNCHECKED` is set.
    pub flags: u8,
}

/// One `[Dirs]` entry: a folder the installer creates even when no file goes into it.
#[derive(Debug, Clone, Default)]
pub struct Directory {
    pub name: String,
    pub conditions: Conditions,
    pub attributes: u32,
    /// Index into the permission entries, or -1.
    pub permission: i16,
    /// See [`directory_flags`].
    pub flags: u8,
}

/// One `[Files]` entry.
#[derive(Debug, Clone, Default)]
pub struct FileEntry {
    /// The source name on the packager's machine (often empty).
    pub source: String,
    /// The full destination name with Inno Setup's constants, e.g. `{app}\cd_image\data\db\x.dat`.
    pub destination: String,
    pub install_font_name: String,
    pub strong_assembly_name: String,
    pub conditions: Conditions,
    /// Index into [`Package::data_entries`]; `None` for an external file or the uninstaller.
    pub location: Option<u32>,
    pub attributes: u32,
    pub external_size: u64,
    /// Index into the permission entries, or -1.
    pub permission: i16,
    /// See [`file_flags`].
    pub flags: u32,
    /// The entry stands for the uninstaller program Inno Setup writes itself (`unins000.exe`), not a packed file.
    pub is_uninstaller: bool,
}

impl FileEntry {
    /// Where the file goes, with the leading constant taken apart.
    pub fn target(&self) -> Destination {
        Destination::parse(&self.destination)
    }
}

/// One packed file: where its bytes are and what they should be.
#[derive(Debug, Clone, Default)]
pub struct DataEntry {
    /// The slice (0 = `install-1.bin`) where the file's chunk starts and the one where it ends.
    pub first_slice: u32,
    pub last_slice: u32,
    /// Where the chunk starts in its first slice file.
    pub chunk_offset: u32,
    /// Where the file starts in the unpacked chunk.
    pub file_offset: u64,
    /// The file's size.
    pub file_size: u64,
    /// The chunk's packed size.
    pub chunk_size: u64,
    /// SHA-1 of the file as installed.
    pub sha1: [u8; 20],
    /// The file's time as a Windows FILETIME (UTC when `TIME_STAMP_IN_UTC` is set, else the packager's local time).
    pub filetime: i64,
    pub file_version: u64,
    /// See [`data_flags`].
    pub flags: u16,
}

impl DataEntry {
    /// The file's time, reading a local-time stamp as if it were UTC (the packager's time zone is not stored).
    /// `None` for a stamp before 1970.
    pub fn modified(&self) -> Option<SystemTime> {
        let since = self.filetime.checked_sub(FILETIME_UNIX_EPOCH)?;
        if since < 0 {
            return None;
        }
        Some(SystemTime::UNIX_EPOCH + Duration::new((since / 10_000_000) as u64, (since % 10_000_000) as u32 * 100))
    }

    /// The file's time in seconds since 1970 (see [`DataEntry::modified`]).
    pub fn unix_time(&self) -> i64 {
        self.filetime.saturating_sub(FILETIME_UNIX_EPOCH).div_euclid(10_000_000)
    }

    /// The SHA-1 as lower-case hex.
    pub fn sha1_hex(&self) -> String {
        hex(&self.sha1)
    }
}

/// One `[Icons]` entry: a shortcut.
#[derive(Debug, Clone, Default)]
pub struct Icon {
    pub name: String,
    pub filename: String,
    pub parameters: String,
    pub working_dir: String,
    pub icon_file: String,
    pub comment: String,
    pub conditions: Conditions,
    /// See [`icon_flags`].
    pub flags: u8,
}

/// One `[INI]` entry: a value the installer writes into an .ini file.
#[derive(Debug, Clone, Default)]
pub struct IniEntry {
    pub file: String,
    pub section: String,
    pub key: String,
    pub value: String,
    pub conditions: Conditions,
    /// See [`ini_flags`].
    pub flags: u8,
}

/// One `[Registry]` entry.
#[derive(Debug, Clone, Default)]
pub struct RegistryEntry {
    /// The root key as its Windows number with the top bit dropped (0 `HKCR`, 1 `HKCU`, 2 `HKLM`, 3 `HKU`, …).
    pub hive: u32,
    pub key: String,
    pub name: String,
    /// The value as stored (ANSI text for strings, raw bytes otherwise).
    pub value: Vec<u8>,
    /// 0 none, 1 string, 2 expand string, 3 dword, 4 binary, 5 multi string, 6 qword.
    pub value_type: u8,
    pub conditions: Conditions,
    pub permission: i16,
    /// See [`registry_flags`].
    pub flags: u16,
}

impl RegistryEntry {
    /// `HKLM` and so on.
    pub fn hive_name(&self) -> &'static str {
        match self.hive {
            0 => "HKCR",
            1 => "HKCU",
            2 => "HKLM",
            3 => "HKU",
            4 => "HKPD",
            5 => "HKCC",
            6 => "HKDD",
            _ => "HK??",
        }
    }
}

/// One `[InstallDelete]` or `[UninstallDelete]` entry.
#[derive(Debug, Clone, Default)]
pub struct DeleteEntry {
    pub name: String,
    pub conditions: Conditions,
    /// 0 files, 1 files and folders below, 2 the folder if empty.
    pub kind: u8,
}

/// One `[Run]` or `[UninstallRun]` entry: a program the installer starts.
#[derive(Debug, Clone, Default)]
pub struct RunEntry {
    pub name: String,
    pub parameters: String,
    pub working_dir: String,
    pub run_once_id: String,
    pub status_message: String,
    pub verb: String,
    pub description: String,
    pub conditions: Conditions,
    pub show_command: i32,
    /// 0 wait until it ends, 1 do not wait, 2 wait until idle.
    pub wait: u8,
    /// See [`run_flags`].
    pub flags: u16,
}

// ---------------------------------------------------------------------------------------------------------------
// Destinations
// ---------------------------------------------------------------------------------------------------------------

/// Which folder a destination starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Root {
    /// `{app}`: the game folder.
    App,
    /// Another Inno Setup constant, by its name without braces (`sys`, `tmp`, `win`, `commonappdata`, `code:…`).
    Other(String),
    /// No constant at the start (an absolute path, or empty).
    None,
}

/// A destination name taken apart: its leading constant and the path below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub root: Root,
    /// The path below the root with `/` between its parts and none at the start.
    pub path: String,
    /// The path still holds a `{constant}` (it depends on the machine or on the installer's script).
    pub has_inner_constant: bool,
}

impl Destination {
    /// Takes a name such as `{app}\cd_image\data\x.dat` apart.
    pub fn parse(name: &str) -> Destination {
        let (root, rest) = match leading_constant(name) {
            Some((constant, rest)) if constant.eq_ignore_ascii_case("app") => (Root::App, rest),
            Some((constant, rest)) => (Root::Other(constant.to_string()), rest),
            None => (Root::None, name),
        };
        let path: String = rest.trim_start_matches(['\\', '/']).replace('\\', "/");
        // (`{{` is a literal brace, not a constant)
        let has_inner_constant = path.replace("{{", "").contains('{');
        Destination { root, path, has_inner_constant }
    }

    /// The path below the game folder, if that is where the name points and the path is safe to create there (no
    /// `..`, no drive, no constant left in it).
    pub fn app_relative(&self) -> Option<&str> {
        (self.root == Root::App && !self.has_inner_constant && safe_relative(&self.path).is_some())
            .then_some(self.path.as_str())
    }
}

/// `{name}rest` → `(name, rest)`, with nested braces (`{code:F|{app}}`) kept whole.
fn leading_constant(name: &str) -> Option<(&str, &str)> {
    if !name.starts_with('{') || name.starts_with("{{") {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in name.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&name[1..i], &name[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// A `/`-separated relative path as a path of this system, or `None` if it would leave the folder it is joined to.
fn safe_relative(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return None,
            p if p.contains([':', '\0']) => return None,
            p => out.push(p),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Gives every folder in the plan one spelling: the first one seen. Windows does not tell `Data\` from `data\`,
/// so a package may spell a folder two ways; on a file system that does tell them apart the files would land in
/// two folders. File names are left as they are.
pub fn unify_folder_case(plan: &mut [PlannedFile]) {
    let mut spelling: HashMap<String, String> = HashMap::new();
    for planned in plan {
        let parts: Vec<&str> = planned.path.split('/').collect();
        let Some((file, folders)) = parts.split_last() else { continue };
        let mut key = String::new();
        let mut out = String::new();
        for folder in folders {
            key.push_str(&folder.to_lowercase());
            key.push('/');
            let chosen = spelling.entry(key.clone()).or_insert_with(|| (*folder).to_string());
            out.push_str(chosen);
            out.push('/');
        }
        out.push_str(file);
        planned.path = out;
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The package
// ---------------------------------------------------------------------------------------------------------------

/// The offset table of the installer program.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OffsetTable {
    /// Where the table was found in the file.
    pub table_offset: u32,
    pub total_size: u32,
    /// The packed setup program (`Setup.e32`) inside the installer: offset, unpacked size and CRC-32.
    pub exe_offset: u32,
    pub exe_unpacked_size: u32,
    pub exe_crc32: u32,
    /// Where the setup data starts.
    pub header_offset: u32,
    /// Where the files' data starts in the installer itself; 0 when it is in slice files beside it.
    pub data_offset: u32,
}

/// An opened Inno Setup 5.4.2 package: everything its setup data says, and the way to its slices.
#[derive(Debug, Clone)]
pub struct Package {
    dir: PathBuf,
    exe_name: String,
    offsets: OffsetTable,
    header: Header,
    languages: Vec<Language>,
    messages: Vec<Message>,
    permissions: Vec<Vec<u8>>,
    types: Vec<SetupType>,
    components: Vec<Component>,
    tasks: Vec<Task>,
    directories: Vec<Directory>,
    files: Vec<FileEntry>,
    icons: Vec<Icon>,
    ini_entries: Vec<IniEntry>,
    registry_entries: Vec<RegistryEntry>,
    install_delete: Vec<DeleteEntry>,
    uninstall_delete: Vec<DeleteEntry>,
    run_entries: Vec<RunEntry>,
    uninstall_run: Vec<RunEntry>,
    data_entries: Vec<DataEntry>,
}

/// One file to unpack: which `[Files]` entry, and where below the destination folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    /// Index into [`Package::files`].
    pub file: usize,
    /// Path below the destination folder, `/` between its parts.
    pub path: String,
}

/// What [`Package::extract`] tells its caller while it works.
#[derive(Debug, Clone, Copy)]
pub struct Progress<'a> {
    /// Unpacked bytes written so far, and the sum for the whole plan.
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Files finished so far, and how many the plan has.
    pub files_done: usize,
    pub files_total: usize,
    /// The file being written (its path below the destination folder).
    pub current: &'a str,
}

/// What [`Package::extract`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Extracted {
    pub files: usize,
    pub bytes: u64,
}

impl Package {
    /// Opens the package in `dir`: the folder that holds `install.exe` and its `install-N.bin` slices.
    pub fn open(dir: impl AsRef<Path>) -> Result<Package> {
        let dir = dir.as_ref();
        match find_ignoring_case(dir, "install.exe") {
            Some(exe) => Package::open_exe(exe),
            None => Err(Error::MissingFile(dir.join("install.exe").display().to_string())),
        }
    }

    /// Opens a package by its installer program; the slices are looked for beside it, named after it.
    pub fn open_exe(exe: impl AsRef<Path>) -> Result<Package> {
        let exe = exe.as_ref();
        let data = fs::read(exe)?;
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        let exe_name = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        Package::parse(&data, dir, exe_name)
    }

    /// Reads the setup data out of the installer program's bytes. `dir` and `exe_name` (the program's name without
    /// `.exe`) say where the slices are.
    pub fn parse(exe: &[u8], dir: PathBuf, exe_name: String) -> Result<Package> {
        let offsets = read_offset_table(exe)?;
        let mut pos = offsets.header_offset as usize;
        let line = exe
            .get(pos..pos.saturating_add(64))
            .ok_or_else(|| Error::Format("setup data lies outside the file".into()))?;
        let end = line.iter().position(|&b| b == 0).unwrap_or(64);
        let version = String::from_utf8_lossy(&line[..end]).into_owned();
        if version != VERSION_LINE {
            return Err(Error::Unsupported(format!(
                "setup data version \"{version}\" (only \"{VERSION_LINE}\" is read)"
            )));
        }
        if offsets.data_offset != 0 {
            return Err(Error::Unsupported("a package with its files inside the installer program".into()));
        }
        pos += 64;
        let first = read_block(exe, &mut pos)?;
        let second = read_block(exe, &mut pos)?;

        let mut cur = Cursor::new(&first);
        let (header, counts) = read_header(&mut cur)?;
        let mut package = Package {
            dir,
            exe_name,
            offsets,
            header,
            languages: read_list(&mut cur, counts.languages, read_language)?,
            messages: read_list(&mut cur, counts.messages, read_message)?,
            permissions: read_list(&mut cur, counts.permissions, |c| Ok(c.binary()?.to_vec()))?,
            types: read_list(&mut cur, counts.types, read_type)?,
            components: read_list(&mut cur, counts.components, read_component)?,
            tasks: read_list(&mut cur, counts.tasks, read_task)?,
            directories: read_list(&mut cur, counts.directories, read_directory)?,
            files: read_list(&mut cur, counts.files, read_file)?,
            icons: read_list(&mut cur, counts.icons, read_icon)?,
            ini_entries: read_list(&mut cur, counts.ini_entries, read_ini)?,
            registry_entries: read_list(&mut cur, counts.registry_entries, read_registry)?,
            install_delete: read_list(&mut cur, counts.install_delete, read_delete)?,
            uninstall_delete: read_list(&mut cur, counts.uninstall_delete, read_delete)?,
            run_entries: read_list(&mut cur, counts.run_entries, read_run)?,
            uninstall_run: read_list(&mut cur, counts.uninstall_run, read_run)?,
            data_entries: Vec::new(),
        };
        // The wizard's two pictures, then the unpacking and decrypting libraries when the package needs them.
        cur.binary()?;
        cur.binary()?;
        if matches!(package.header.compression, Some(Compression::BZip2 | Compression::Zlib)) {
            cur.binary()?;
        }
        if package.header.flags & header_flags::ENCRYPTION_USED != 0 {
            cur.binary()?;
        }
        if cur.left() != 0 {
            return format_err(format!("{} bytes left over after the setup data's lists", cur.left()));
        }

        let mut cur = Cursor::new(&second);
        package.data_entries = read_list(&mut cur, counts.data_entries, read_data_entry)?;
        if cur.left() != 0 {
            return format_err(format!("{} bytes left over after the data entries", cur.left()));
        }
        for (i, file) in package.files.iter().enumerate() {
            if let Some(location) = file.location
                && location as usize >= package.data_entries.len()
            {
                return format_err(format!("file entry {i} names data entry {location}, which does not exist"));
            }
        }
        Ok(package)
    }

    /// The folder the package was opened from.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The installer program's offset table.
    pub fn offsets(&self) -> &OffsetTable {
        &self.offsets
    }

    /// The `[Setup]` section.
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Access rules (`Permissions:`) some entries name by index: raw Windows records.
    pub fn permissions(&self) -> &[Vec<u8>] {
        &self.permissions
    }

    pub fn types(&self) -> &[SetupType] {
        &self.types
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// `[Dirs]`: folders created even if empty.
    pub fn directories(&self) -> &[Directory] {
        &self.directories
    }

    /// `[Files]`, in the script's order.
    pub fn files(&self) -> &[FileEntry] {
        &self.files
    }

    /// The packed files, as the file entries name them by index.
    pub fn data_entries(&self) -> &[DataEntry] {
        &self.data_entries
    }

    /// `[Icons]`: shortcuts.
    pub fn icons(&self) -> &[Icon] {
        &self.icons
    }

    /// `[INI]`: values written into .ini files.
    pub fn ini_entries(&self) -> &[IniEntry] {
        &self.ini_entries
    }

    /// `[Registry]`.
    pub fn registry_entries(&self) -> &[RegistryEntry] {
        &self.registry_entries
    }

    /// `[InstallDelete]`: what the installer removes before copying.
    pub fn install_delete(&self) -> &[DeleteEntry] {
        &self.install_delete
    }

    /// `[UninstallDelete]`.
    pub fn uninstall_delete(&self) -> &[DeleteEntry] {
        &self.uninstall_delete
    }

    /// `[Run]`: programs started after copying.
    pub fn run_entries(&self) -> &[RunEntry] {
        &self.run_entries
    }

    /// `[UninstallRun]`.
    pub fn uninstall_run(&self) -> &[RunEntry] {
        &self.uninstall_run
    }

    /// The packed file behind a `[Files]` entry.
    pub fn data_of(&self, file: &FileEntry) -> Option<&DataEntry> {
        self.data_entries.get(file.location? as usize)
    }

    /// The slice files the package needs, in order (`install-1.bin`, …), as they should be named.
    pub fn slice_names(&self) -> Vec<String> {
        let last = self.data_entries.iter().map(|d| d.last_slice.max(d.first_slice)).max();
        match last {
            Some(last) => (0..=last).map(|n| slice_name(&self.exe_name, n, self.header.slices_per_disk)).collect(),
            None => Vec::new(),
        }
    }

    /// Whether a component is part of a default install: its `types` name the default setup type (the first one),
    /// or it is fixed. With no setup types at all every component is taken.
    pub fn component_is_default(&self, name: &str) -> bool {
        let Some(component) = self.components.iter().find(|c| c.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        if component.flags & component_flags::FIXED != 0 {
            return true;
        }
        match self.types.first() {
            Some(default) => component.types.split_whitespace().any(|t| t.eq_ignore_ascii_case(&default.name)),
            None => true,
        }
    }

    /// Whether a task is ticked in a default install (not flagged `unchecked`, and its components are taken).
    pub fn task_is_default(&self, name: &str) -> bool {
        let Some(task) = self.tasks.iter().find(|t| t.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        task.flags & task_flags::UNCHECKED == 0
            && eval_expression(&task.components, &|c| self.component_is_default(c)).unwrap_or(true)
    }

    /// Whether an entry's conditions hold in a default install (default setup type, default tasks, first language).
    /// `None` when that cannot be told: the entry has a `Check:` that only the installer's script can answer, or
    /// an expression this reader cannot parse.
    pub fn default_install(&self, conditions: &Conditions) -> Option<bool> {
        let components = eval_expression(&conditions.components, &|c| self.component_is_default(c))?;
        let tasks = eval_expression(&conditions.tasks, &|t| self.task_is_default(t))?;
        let first_language = self.languages.first().map(|l| l.name.as_str()).unwrap_or("default");
        let languages = eval_expression(&conditions.languages, &|l| l.eq_ignore_ascii_case(first_language))?;
        if !(components && tasks && languages) {
            return Some(false);
        }
        conditions.check.is_empty().then_some(true)
    }

    /// The files a default install puts into the game folder, with their paths below it: every `[Files]` entry that
    /// is packed in the slices, goes under `{app}`, is copied at all (not `dontcopy`, not removed again by
    /// `deleteafterinstall`) and whose conditions hold by default ([`Package::default_install`]; entries with a
    /// script `Check:` are left out and can be found with [`Package::files`]). Folders get one spelling
    /// ([`unify_folder_case`]).
    pub fn plan_app(&self) -> Vec<PlannedFile> {
        let mut plan = self.plan(|file, target| {
            let skip = file_flags::DONT_COPY | file_flags::DELETE_AFTER_INSTALL;
            if file.flags & skip != 0 || self.default_install(&file.conditions) != Some(true) {
                return None;
            }
            target.app_relative().map(str::to_string)
        });
        unify_folder_case(&mut plan);
        plan
    }

    /// Builds a plan from a choice per packed file: `choose` gets each `[Files]` entry that has data in the slices
    /// and its destination taken apart, and gives the path below the destination folder or `None` to leave it out.
    pub fn plan(&self, mut choose: impl FnMut(&FileEntry, &Destination) -> Option<String>) -> Vec<PlannedFile> {
        let mut plan = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            if file.location.is_none() {
                continue;
            }
            if let Some(path) = choose(file, &file.target()) {
                plan.push(PlannedFile { file: index, path });
            }
        }
        plan
    }

    /// The unpacked size of a plan in bytes.
    pub fn plan_size(&self, plan: &[PlannedFile]) -> u64 {
        plan.iter().filter_map(|p| self.files.get(p.file)).filter_map(|f| self.data_of(f)).map(|d| d.file_size).sum()
    }

    /// Unpacks the planned files into `dest`, checking each against its SHA-1.
    ///
    /// Memory use is bounded (the 8 MB LZMA dictionary and a few buffers) whatever the files' sizes. Each file is
    /// written under a temporary name (`….mao-part`) and renamed once its check sum is right, so a file with its
    /// real name is always whole; its modification time is set from the package. When two planned files have the
    /// same path (whatever the case of its letters) the later one wins, as in the installer. `progress` is called
    /// at the start of each file and after every buffer written; when it answers `false` the unpacking stops with
    /// [`Error::Stopped`], the unfinished file taken away.
    ///
    /// Files that share a chunk are unpacked in one pass; a chunk no planned file lies in is not read at all, and
    /// reading stops after a chunk's last wanted file.
    pub fn extract(
        &self,
        dest: impl AsRef<Path>,
        plan: &[PlannedFile],
        progress: &mut dyn FnMut(Progress<'_>) -> bool,
    ) -> Result<Extracted> {
        let dest = dest.as_ref();
        // Later entries for one path replace earlier ones.
        let mut last_for_path: HashMap<String, usize> = HashMap::new();
        for (i, planned) in plan.iter().enumerate() {
            last_for_path.insert(planned.path.to_lowercase(), i);
        }
        // Data entry → the places its bytes go.
        let mut wanted: BTreeMap<u32, Vec<&PlannedFile>> = BTreeMap::new();
        let mut bytes_total = 0u64;
        let mut files_total = 0usize;
        for (i, planned) in plan.iter().enumerate() {
            if last_for_path.get(&planned.path.to_lowercase()) != Some(&i) {
                continue;
            }
            let file = self.files.get(planned.file).ok_or_else(|| Error::Format("plan names a missing file".into()))?;
            let location =
                file.location.ok_or_else(|| Error::Format(format!("{} is not in the package", planned.path)))?;
            if safe_relative(&planned.path).is_none() {
                return format_err(format!("unsafe path in plan: {}", planned.path));
            }
            wanted.entry(location).or_default().push(planned);
            bytes_total += self.data_entries[location as usize].file_size;
            files_total += 1;
        }
        // Chunk → its wanted data entries.
        let mut chunks: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
        for &location in wanted.keys() {
            let data = &self.data_entries[location as usize];
            chunks.entry((data.first_slice, data.chunk_offset)).or_default().push(location);
        }

        let mut slices = SliceReader::new(&self.dir, &self.exe_name, self.header.slices_per_disk);
        let mut done = Extracted::default();
        let mut buffer = vec![0u8; COPY_BUFFER + 4];
        for ((first_slice, chunk_offset), mut locations) in chunks {
            // In the order the files lie in the unpacked chunk.
            locations.sort_by_key(|&l| self.data_entries[l as usize].file_offset);
            let head = &self.data_entries[locations[0] as usize];
            if head.flags & data_flags::CHUNK_ENCRYPTED != 0 {
                return Err(Error::Unsupported("the package's files are encrypted".into()));
            }
            let compression = if head.flags & data_flags::CHUNK_COMPRESSED != 0 {
                self.header.compression.ok_or_else(|| Error::Unsupported("unknown compression method".into()))?
            } else {
                Compression::Stored
            };
            slices.seek(first_slice, chunk_offset)?;
            let mut magic = [0u8; 4];
            slices.read_exact(&mut magic)?;
            if &magic != CHUNK_MAGIC {
                return format_err(format!("no chunk at offset {chunk_offset} of slice {}", first_slice + 1));
            }
            let packed = ChunkReader { slices: &mut slices, left: head.chunk_size };
            let mut stream: Box<dyn Read + '_> = match compression {
                Compression::Stored => Box::new(packed),
                Compression::Lzma1 => Box::new(Lzma1Reader::new(packed)?),
                other => {
                    return Err(Error::Unsupported(format!(
                        "{other:?} compression (this reader unpacks LZMA1 and stored chunks only)"
                    )));
                }
            };
            let mut at = 0u64;
            for location in locations {
                let data = &self.data_entries[location as usize];
                if data.file_offset < at {
                    return format_err("two packed files overlap in their chunk");
                }
                skip(&mut stream, data.file_offset - at, &mut buffer)?;
                let targets = &wanted[&location];
                write_file(dest, data, targets, &mut stream, &mut buffer, &mut |current, wrote| {
                    done.bytes += wrote;
                    progress(Progress {
                        bytes_done: done.bytes,
                        bytes_total,
                        files_done: done.files,
                        files_total,
                        current,
                    })
                })?;
                done.files += targets.len();
                at = data.file_offset + data.file_size;
            }
        }
        Ok(done)
    }
}

/// Unpacks one packed file from `stream` (standing at its first byte) to every target, checks it and gives it its
/// name and time.
fn write_file(
    dest: &Path,
    data: &DataEntry,
    targets: &[&PlannedFile],
    stream: &mut dyn Read,
    buffer: &mut [u8],
    wrote: &mut dyn FnMut(&str, u64) -> bool,
) -> Result<()> {
    let first = targets[0].path.as_str();
    let mut outputs = Vec::with_capacity(targets.len());
    for target in targets {
        let relative = safe_relative(&target.path).ok_or_else(|| Error::Format("unsafe path".into()))?;
        let path = dest.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut part = path.clone().into_os_string();
        part.push(".mao-part");
        let part = PathBuf::from(part);
        let file = File::create(&part)?;
        outputs.push((file, part, path));
    }
    let mut going = wrote(first, 0);

    let mut filter = (data.flags & data_flags::CALL_INSTRUCTION_OPTIMIZED != 0).then(CallFilter::new);
    let mut sum = Sha1::new();
    let mut left = data.file_size;
    // Bytes at the front of the buffer that the filter could not finish without seeing what follows (at most 4).
    let mut held = 0usize;
    while going && (left > 0 || held > 0) {
        let want = (left.min(COPY_BUFFER as u64) as usize).min(buffer.len() - held);
        stream.read_exact(&mut buffer[held..held + want]).map_err(|e| match e.kind() {
            io::ErrorKind::UnexpectedEof => Error::Format(format!("the package ends inside {first}")),
            _ => Error::from(e),
        })?;
        left -= want as u64;
        let have = held + want;
        let ready = match &mut filter {
            Some(filter) => filter.decode(&mut buffer[..have], left == 0),
            None => have,
        };
        sum.update(&buffer[..ready]);
        for (file, _, _) in &mut outputs {
            file.write_all(&buffer[..ready])?;
        }
        buffer.copy_within(ready..have, 0);
        held = have - ready;
        going = wrote(first, (ready * outputs.len()) as u64);
    }
    if !going {
        for (file, part, _) in outputs {
            drop(file);
            let _ = fs::remove_file(part);
        }
        return Err(Error::Stopped);
    }
    let actual = sum.finish();
    if actual != data.sha1 {
        for (file, part, _) in outputs {
            drop(file);
            let _ = fs::remove_file(part);
        }
        return Err(Error::Checksum { what: first.to_string(), expected: hex(&data.sha1), actual: hex(&actual) });
    }
    let modified = data.modified();
    for (file, part, path) in outputs {
        if let Some(time) = modified {
            // (a file system that cannot hold the time is no reason to fail the install)
            let _ = file.set_modified(time);
        }
        drop(file);
        fs::rename(&part, &path)?;
    }
    Ok(())
}

/// Reads and throws away `count` bytes.
fn skip(stream: &mut dyn Read, mut count: u64, buffer: &mut [u8]) -> Result<()> {
    while count > 0 {
        let n = count.min(buffer.len() as u64) as usize;
        stream.read_exact(&mut buffer[..n])?;
        count -= n as u64;
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Evaluates a `Components:`/`Tasks:`/`Languages:` expression: names joined by `and`, `or`, `not` and brackets;
/// names side by side mean "or". An empty expression is true. `None` if it cannot be parsed.
fn eval_expression(text: &str, is_set: &dyn Fn(&str) -> bool) -> Option<bool> {
    type Test<'a> = &'a dyn Fn(&str) -> bool;
    fn any_of(tokens: &[String], at: &mut usize, is_set: Test<'_>) -> Option<bool> {
        let mut value = all_of(tokens, at, is_set)?;
        loop {
            match tokens.get(*at).map(String::as_str) {
                None | Some(")") => return Some(value),
                Some(t) if t.eq_ignore_ascii_case("or") => *at += 1,
                Some(_) => {}
            }
            value |= all_of(tokens, at, is_set)?;
        }
    }
    fn all_of(tokens: &[String], at: &mut usize, is_set: Test<'_>) -> Option<bool> {
        let mut value = one(tokens, at, is_set)?;
        while tokens.get(*at).is_some_and(|t| t.eq_ignore_ascii_case("and")) {
            *at += 1;
            value &= one(tokens, at, is_set)?;
        }
        Some(value)
    }
    fn one(tokens: &[String], at: &mut usize, is_set: Test<'_>) -> Option<bool> {
        let token = tokens.get(*at)?;
        *at += 1;
        if token.eq_ignore_ascii_case("not") {
            return one(tokens, at, is_set).map(|v| !v);
        }
        if token == "(" {
            let value = any_of(tokens, at, is_set)?;
            if tokens.get(*at).map(String::as_str) != Some(")") {
                return None;
            }
            *at += 1;
            return Some(value);
        }
        if token == ")" || token.eq_ignore_ascii_case("and") || token.eq_ignore_ascii_case("or") {
            return None;
        }
        Some(is_set(token))
    }

    if text.trim().is_empty() {
        return Some(true);
    }
    let mut tokens = Vec::new();
    let mut word = String::new();
    for c in text.chars().chain([' ']) {
        if c.is_whitespace() || c == '(' || c == ')' {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            if c == '(' || c == ')' {
                tokens.push(c.to_string());
            }
        } else {
            word.push(c);
        }
    }
    let mut at = 0;
    let value = any_of(&tokens, &mut at, is_set)?;
    (at == tokens.len()).then_some(value)
}

// ---------------------------------------------------------------------------------------------------------------
// The installer program: offset table and blocks
// ---------------------------------------------------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(data);
    crc.sum()
}

fn crc_error(what: &str, expected: u32, actual: u32) -> Error {
    Error::Checksum { what: what.into(), expected: format!("{expected:08x}"), actual: format!("{actual:08x}") }
}

fn le16(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at.checked_add(2)?).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32)
}

fn le32(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at.checked_add(4)?).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Finds and checks the offset table: at the place the file's `Inno` mark at 0x30 names (older loaders), else in
/// the program's resources as `RCDATA` number 11111.
fn read_offset_table(exe: &[u8]) -> Result<OffsetTable> {
    let mut at = None;
    if le32(exe, 0x30) == Some(0x6f6e_6e49)
        && let (Some(offset), Some(inverse)) = (le32(exe, 0x34), le32(exe, 0x38))
        && offset == !inverse
    {
        at = Some(offset as usize);
    }
    let Some(at) = at.or_else(|| find_rcdata(exe, 11111)) else {
        return format_err("no Inno Setup offset table in the installer program");
    };
    let table = exe
        .get(at..at.saturating_add(44))
        .ok_or_else(|| Error::Format("offset table lies outside the file".into()))?;
    if table[..12] != OFFSET_TABLE_MAGIC {
        return Err(Error::Unsupported("offset table of another Inno Setup version".into()));
    }
    let word = |i: usize| u32::from_le_bytes([table[i], table[i + 1], table[i + 2], table[i + 3]]);
    if word(12) != 1 {
        return Err(Error::Unsupported(format!("offset table revision {}", word(12))));
    }
    let (expected, actual) = (word(40), crc32(&table[..40]));
    if expected != actual {
        return Err(crc_error("the installer's offset table", expected, actual));
    }
    Ok(OffsetTable {
        table_offset: at as u32,
        total_size: word(16),
        exe_offset: word(20),
        exe_unpacked_size: word(24),
        exe_crc32: word(28),
        header_offset: word(32),
        data_offset: word(36),
    })
}

/// The file offset of the `RCDATA` resource with the given number in a Windows program, if it has one.
fn find_rcdata(exe: &[u8], id: u32) -> Option<usize> {
    if exe.get(..2)? != b"MZ" {
        return None;
    }
    let pe = le32(exe, 0x3c)? as usize;
    if exe.get(pe..pe.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }
    let section_count = le16(exe, pe + 6)? as usize;
    let optional = pe + 24;
    let optional_size = le16(exe, pe + 20)? as usize;
    // The data directories follow 96 bytes of optional header in a 32-bit program, 112 in a 64-bit one; the
    // resources are the third directory.
    let directories = match le16(exe, optional)? {
        0x10b => optional + 96,
        0x20b => optional + 112,
        _ => return None,
    };
    let resources_rva = le32(exe, directories + 2 * 8)?;
    let sections = optional + optional_size;
    let to_offset = |rva: u32| -> Option<usize> {
        for i in 0..section_count {
            let s = sections + i * 40;
            let (virtual_size, address) = (le32(exe, s + 8)?, le32(exe, s + 12)?);
            let (raw_size, raw) = (le32(exe, s + 16)?, le32(exe, s + 20)?);
            if rva >= address && rva - address < virtual_size.max(raw_size) {
                return Some(raw as usize + (rva - address) as usize);
            }
        }
        None
    };
    let base = to_offset(resources_rva)?;
    // Three levels: type (10 = RCDATA), name, language. An entry is (id, offset from `base`); the offset's top bit
    // marks a further level. `want` of `None` takes the first entry.
    let find = |dir: usize, want: Option<u32>| -> Option<u32> {
        let named = le16(exe, dir.checked_add(12)?)? as usize;
        let numbered = le16(exe, dir.checked_add(14)?)? as usize;
        for i in 0..named + numbered {
            let entry = dir + 16 + i * 8;
            let (name, offset) = (le32(exe, entry)?, le32(exe, entry + 4)?);
            if want.is_none() || (i >= named && want == Some(name)) {
                return Some(offset);
            }
        }
        None
    };
    let types = find(base, Some(10))?;
    let names = find(base + (types & 0x7fff_ffff) as usize, Some(id))?;
    let leaf = if names & 0x8000_0000 != 0 { find(base + (names & 0x7fff_ffff) as usize, None)? } else { names };
    if leaf & 0x8000_0000 != 0 {
        return None;
    }
    to_offset(le32(exe, base + leaf as usize)?)
}

/// Reads one block of the setup data at `pos`, checks its CRC-32s and unpacks it; `pos` moves past it.
fn read_block(exe: &[u8], pos: &mut usize) -> Result<Vec<u8>> {
    let head = exe
        .get(*pos..pos.saturating_add(9))
        .ok_or_else(|| Error::Format("setup data block lies outside the file".into()))?;
    let expected = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let actual = crc32(&head[4..9]);
    if expected != actual {
        return Err(crc_error("a setup data block's head", expected, actual));
    }
    let stored = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let packed = head[8] != 0;
    let body = exe
        .get(*pos + 9..(*pos + 9).saturating_add(stored))
        .ok_or_else(|| Error::Format("setup data block runs past the end of the file".into()))?;
    *pos += 9 + stored;
    // Pieces of up to 4096 bytes, each led by its CRC-32.
    let mut joined = Vec::with_capacity(stored);
    for piece in body.chunks(4 + 4096) {
        if piece.len() < 4 {
            return format_err("setup data block ends inside a check sum");
        }
        let expected = u32::from_le_bytes([piece[0], piece[1], piece[2], piece[3]]);
        let actual = crc32(&piece[4..]);
        if expected != actual {
            return Err(crc_error("a piece of the setup data", expected, actual));
        }
        joined.extend_from_slice(&piece[4..]);
    }
    if !packed {
        return Ok(joined);
    }
    // The stream carries no size: it ends with LZMA's end marker.
    let mut reader = Lzma1Reader::new(&joined[..]).map_err(|e| Error::Format(format!("setup data: {e}")))?;
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(|e| Error::Format(format!("setup data: {e}")))?;
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// Reading the setup data's records
// ---------------------------------------------------------------------------------------------------------------

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    fn left(&self) -> usize {
        self.data.len() - self.pos
    }

    fn bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        if count > self.left() {
            return format_err(format!(
                "setup data ends at byte {}, an entry at {} wants {count} more",
                self.data.len(),
                self.pos
            ));
        }
        let out = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.bytes(N)?.try_into().expect("length checked"))
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    /// A length-led run of bytes.
    fn binary(&mut self) -> Result<&'a [u8]> {
        let len = self.u32()? as usize;
        self.bytes(len)
    }

    /// A length-led ANSI string, read as Windows code page 1252 (which the package's language uses; plain ASCII
    /// reads the same in every ANSI code page).
    fn string(&mut self) -> Result<String> {
        Ok(decode_windows_1252(self.binary()?))
    }

    /// A stored set of `bits` flags: one byte per eight, three bytes padded to four (as Delphi lays sets out).
    fn flags(&mut self, bits: u32) -> Result<u64> {
        let count = bits.div_ceil(8) as usize;
        let mut value = 0u64;
        for (i, &b) in self.bytes(count)?.iter().enumerate() {
            value |= (b as u64) << (8 * i);
        }
        if count == 3 {
            self.u8()?;
        }
        Ok(value)
    }

    fn windows_version(&mut self) -> Result<WindowsVersion> {
        let one = |c: &mut Self| -> Result<(u8, u8, u16)> {
            let build = c.u16()?;
            let minor = c.u8()?;
            let major = c.u8()?;
            Ok((major, minor, build))
        };
        let win = one(self)?;
        let nt = one(self)?;
        let sp_minor = self.u8()?;
        let sp_major = self.u8()?;
        Ok(WindowsVersion { win, nt, service_pack: (sp_major, sp_minor) })
    }

    /// The six condition strings every entry has. The version limits follow elsewhere in the entry.
    fn conditions(&mut self) -> Result<Conditions> {
        Ok(Conditions {
            components: self.string()?,
            tasks: self.string()?,
            languages: self.string()?,
            check: self.string()?,
            after_install: self.string()?,
            before_install: self.string()?,
            ..Conditions::default()
        })
    }

    fn version_limits(&mut self, conditions: &mut Conditions) -> Result<()> {
        conditions.min_version = self.windows_version()?;
        conditions.only_below_version = self.windows_version()?;
        Ok(())
    }
}

/// Bytes of Windows code page 1252 as text.
fn decode_windows_1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2c6}',
        '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}', '\u{90}', '\u{2018}', '\u{2019}',
        '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}',
        '\u{153}', '\u{9d}', '\u{17e}', '\u{178}',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9f => HIGH[(b - 0x80) as usize],
            _ => b as char,
        })
        .collect()
}

fn read_list<T>(cur: &mut Cursor<'_>, count: u32, read: impl Fn(&mut Cursor<'_>) -> Result<T>) -> Result<Vec<T>> {
    // (the count is not trusted to size the list: each entry takes bytes, and the data is finite)
    let mut list = Vec::new();
    for _ in 0..count {
        list.push(read(cur)?);
    }
    Ok(list)
}

#[derive(Default)]
struct Counts {
    languages: u32,
    messages: u32,
    permissions: u32,
    types: u32,
    components: u32,
    tasks: u32,
    directories: u32,
    files: u32,
    data_entries: u32,
    icons: u32,
    ini_entries: u32,
    registry_entries: u32,
    install_delete: u32,
    uninstall_delete: u32,
    run_entries: u32,
    uninstall_run: u32,
}

fn read_header(c: &mut Cursor<'_>) -> Result<(Header, Counts)> {
    let mut h = Header {
        app_name: c.string()?,
        app_versioned_name: c.string()?,
        app_id: c.string()?,
        app_copyright: c.string()?,
        app_publisher: c.string()?,
        app_publisher_url: c.string()?,
        app_support_phone: c.string()?,
        app_support_url: c.string()?,
        app_updates_url: c.string()?,
        app_version: c.string()?,
        default_dir_name: c.string()?,
        default_group_name: c.string()?,
        base_filename: c.string()?,
        uninstall_files_dir: c.string()?,
        uninstall_name: c.string()?,
        uninstall_icon: c.string()?,
        app_mutex: c.string()?,
        default_user_name: c.string()?,
        default_user_organisation: c.string()?,
        default_serial: c.string()?,
        app_readme_file: c.string()?,
        app_contact: c.string()?,
        app_comments: c.string()?,
        app_modify_path: c.string()?,
        create_uninstall_registry_key: c.string()?,
        uninstallable: c.string()?,
        license_text: c.string()?,
        info_before: c.string()?,
        info_after: c.string()?,
        compiled_code: c.binary()?.to_vec(),
        ..Header::default()
    };
    // Which bytes start a two-byte character in the packager's code page (256 bits): none in code page 1252.
    c.bytes(32)?;
    let counts = Counts {
        languages: c.u32()?,
        messages: c.u32()?,
        permissions: c.u32()?,
        types: c.u32()?,
        components: c.u32()?,
        tasks: c.u32()?,
        directories: c.u32()?,
        files: c.u32()?,
        data_entries: c.u32()?,
        icons: c.u32()?,
        ini_entries: c.u32()?,
        registry_entries: c.u32()?,
        install_delete: c.u32()?,
        uninstall_delete: c.u32()?,
        run_entries: c.u32()?,
        uninstall_run: c.u32()?,
    };
    h.min_version = c.windows_version()?;
    h.only_below_version = c.windows_version()?;
    c.bytes(12)?; // the wizard's background colours
    h.password_hash = c.array()?;
    h.password_salt = c.array()?;
    h.extra_disk_space_required = c.i64()?;
    h.slices_per_disk = c.u32()?;
    c.u8()?; // uninstall log mode
    c.u8()?; // "folder exists" warning
    h.privileges_required = c.u8()?;
    c.u8()?; // show the language dialog
    c.u8()?; // language detection
    h.compression = match c.u8()? {
        0 => Some(Compression::Stored),
        1 => Some(Compression::Zlib),
        2 => Some(Compression::BZip2),
        3 => Some(Compression::Lzma1),
        4 => Some(Compression::Lzma2),
        _ => None,
    };
    c.u8()?; // architectures allowed
    c.u8()?; // architectures installed in 64-bit mode
    c.u8()?; // disable the folder page
    c.u8()?; // disable the program group page
    h.uninstall_display_size = c.u32()?;
    h.flags = c.flags(header_flags::COUNT)?;
    if h.slices_per_disk == 0 {
        return format_err("slices per disk is zero");
    }
    Ok((h, counts))
}

fn read_language(c: &mut Cursor<'_>) -> Result<Language> {
    let name = c.string()?;
    // The language's own name is UTF-16.
    let units: Vec<u16> = c.binary()?.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect();
    let language_name = String::from_utf16_lossy(&units);
    for _ in 0..4 {
        c.binary()?; // dialog, title, welcome and copyright fonts
    }
    c.binary()?; // the language's message file
    for _ in 0..3 {
        c.binary()?; // licence, "before" and "after" texts
    }
    let language_id = c.u32()?;
    let codepage = c.u32()?;
    c.bytes(16)?; // four font sizes
    c.u8()?; // right to left
    Ok(Language { name, language_name, language_id, codepage })
}

fn read_message(c: &mut Cursor<'_>) -> Result<Message> {
    Ok(Message { name: c.string()?, value: c.string()?, language: c.i32()? })
}

fn read_type(c: &mut Cursor<'_>) -> Result<SetupType> {
    let mut t = SetupType {
        name: c.string()?,
        description: c.string()?,
        languages: c.string()?,
        check: c.string()?,
        ..SetupType::default()
    };
    c.bytes(20)?; // Windows version limits
    t.custom = c.flags(1)? & 1 != 0;
    c.u8()?; // user / default full / default compact / default custom
    t.size = c.u64()?;
    Ok(t)
}

fn read_component(c: &mut Cursor<'_>) -> Result<Component> {
    let mut x = Component {
        name: c.string()?,
        description: c.string()?,
        types: c.string()?,
        languages: c.string()?,
        check: c.string()?,
        extra_disk_space_required: c.u64()?,
        level: c.i32()?,
        used: c.u8()? != 0,
        ..Component::default()
    };
    c.bytes(20)?;
    x.flags = c.flags(component_flags::COUNT)? as u8;
    x.size = c.u64()?;
    Ok(x)
}

fn read_task(c: &mut Cursor<'_>) -> Result<Task> {
    let mut t = Task {
        name: c.string()?,
        description: c.string()?,
        group_description: c.string()?,
        components: c.string()?,
        languages: c.string()?,
        check: c.string()?,
        level: c.i32()?,
        used: c.u8()? != 0,
        ..Task::default()
    };
    c.bytes(20)?;
    t.flags = c.flags(task_flags::COUNT)? as u8;
    Ok(t)
}

fn read_directory(c: &mut Cursor<'_>) -> Result<Directory> {
    let mut d = Directory { name: c.string()?, conditions: c.conditions()?, ..Directory::default() };
    d.attributes = c.u32()?;
    c.version_limits(&mut d.conditions)?;
    d.permission = c.u16()? as i16;
    d.flags = c.flags(directory_flags::COUNT)? as u8;
    Ok(d)
}

fn read_file(c: &mut Cursor<'_>) -> Result<FileEntry> {
    let mut f = FileEntry {
        source: c.string()?,
        destination: c.string()?,
        install_font_name: c.string()?,
        strong_assembly_name: c.string()?,
        conditions: c.conditions()?,
        ..FileEntry::default()
    };
    c.version_limits(&mut f.conditions)?;
    let location = c.u32()?;
    f.location = (location != u32::MAX).then_some(location);
    f.attributes = c.u32()?;
    f.external_size = c.u64()?;
    f.permission = c.u16()? as i16;
    f.flags = c.flags(file_flags::COUNT)? as u32;
    f.is_uninstaller = match c.u8()? {
        0 => false,
        1 => true,
        other => return format_err(format!("file entry of unknown kind {other}")),
    };
    Ok(f)
}

fn read_icon(c: &mut Cursor<'_>) -> Result<Icon> {
    let mut i = Icon {
        name: c.string()?,
        filename: c.string()?,
        parameters: c.string()?,
        working_dir: c.string()?,
        icon_file: c.string()?,
        comment: c.string()?,
        conditions: c.conditions()?,
        ..Icon::default()
    };
    c.binary()?; // AppUserModelID
    c.version_limits(&mut i.conditions)?;
    c.bytes(4 + 4 + 1 + 2)?; // icon index, show command, close-on-exit, hot key
    i.flags = c.flags(icon_flags::COUNT)? as u8;
    Ok(i)
}

fn read_ini(c: &mut Cursor<'_>) -> Result<IniEntry> {
    let mut i = IniEntry {
        file: c.string()?,
        section: c.string()?,
        key: c.string()?,
        value: c.string()?,
        conditions: c.conditions()?,
        ..IniEntry::default()
    };
    c.version_limits(&mut i.conditions)?;
    i.flags = c.flags(ini_flags::COUNT)? as u8;
    Ok(i)
}

fn read_registry(c: &mut Cursor<'_>) -> Result<RegistryEntry> {
    let mut r = RegistryEntry {
        key: c.string()?,
        name: c.string()?,
        value: c.binary()?.to_vec(),
        conditions: c.conditions()?,
        ..RegistryEntry::default()
    };
    c.version_limits(&mut r.conditions)?;
    r.hive = c.u32()? & 0x7fff_ffff;
    r.permission = c.u16()? as i16;
    r.value_type = c.u8()?;
    r.flags = c.flags(registry_flags::COUNT)? as u16;
    Ok(r)
}

fn read_delete(c: &mut Cursor<'_>) -> Result<DeleteEntry> {
    let mut d = DeleteEntry { name: c.string()?, conditions: c.conditions()?, kind: 0 };
    c.version_limits(&mut d.conditions)?;
    d.kind = c.u8()?;
    Ok(d)
}

fn read_run(c: &mut Cursor<'_>) -> Result<RunEntry> {
    let mut r = RunEntry {
        name: c.string()?,
        parameters: c.string()?,
        working_dir: c.string()?,
        run_once_id: c.string()?,
        status_message: c.string()?,
        verb: c.string()?,
        description: c.string()?,
        conditions: c.conditions()?,
        ..RunEntry::default()
    };
    c.version_limits(&mut r.conditions)?;
    r.show_command = c.i32()?;
    r.wait = c.u8()?;
    r.flags = c.flags(run_flags::COUNT)? as u16;
    Ok(r)
}

fn read_data_entry(c: &mut Cursor<'_>) -> Result<DataEntry> {
    Ok(DataEntry {
        first_slice: c.u32()?,
        last_slice: c.u32()?,
        chunk_offset: c.u32()?,
        file_offset: c.u64()?,
        file_size: c.u64()?,
        chunk_size: c.u64()?,
        sha1: c.array()?,
        filetime: c.i64()?,
        file_version: ((c.u32()? as u64) << 32) | c.u32()? as u64,
        flags: c.flags(data_flags::COUNT)? as u16,
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Slices and chunks
// ---------------------------------------------------------------------------------------------------------------

/// `install-1.bin` for slice 0; with several slices per disk, `install-1a.bin`, `install-1b.bin`, ….
fn slice_name(base: &str, slice: u32, per_disk: u32) -> String {
    if per_disk <= 1 {
        format!("{base}-{}.bin", slice + 1)
    } else {
        let letter = (b'a' + (slice % per_disk).min(25) as u8) as char;
        format!("{base}-{}{letter}.bin", slice / per_disk + 1)
    }
}

/// A file in `dir` by name, whatever the case of its letters (a download on Linux may not keep Windows' case).
fn find_ignoring_case(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Reads the slices as one run of bytes: from a place in one slice on into the next ones.
struct SliceReader {
    dir: PathBuf,
    base: String,
    per_disk: u32,
    open: Option<OpenSlice>,
}

struct OpenSlice {
    number: u32,
    file: File,
    /// The slice's size as its head gives it, and the read position.
    size: u64,
    pos: u64,
}

impl SliceReader {
    fn new(dir: &Path, base: &str, per_disk: u32) -> Self {
        SliceReader { dir: dir.to_path_buf(), base: base.to_string(), per_disk, open: None }
    }

    /// Opens a slice and checks its head; the position is left after the head.
    fn open(&mut self, number: u32) -> Result<&mut OpenSlice> {
        let name = slice_name(&self.base, number, self.per_disk);
        let path = find_ignoring_case(&self.dir, &name)
            .ok_or_else(|| Error::MissingFile(self.dir.join(&name).display().to_string()))?;
        let mut file = File::open(&path)?;
        let length = file.metadata()?.len();
        let mut head = [0u8; 12];
        file.read_exact(&mut head).map_err(|_| Error::Format(format!("{name} is too short to be a slice")))?;
        if &head[..8] != SLICE_MAGIC {
            return format_err(format!("{name} is not an Inno Setup slice"));
        }
        let size = u32::from_le_bytes([head[8], head[9], head[10], head[11]]) as u64;
        if size > length {
            return format_err(format!("{name} is {length} bytes long, should be {size}: incomplete download?"));
        }
        if size < 12 {
            return format_err(format!("{name} gives an impossible size"));
        }
        Ok(self.open.insert(OpenSlice { number, file, size, pos: 12 }))
    }

    fn seek(&mut self, number: u32, offset: u32) -> Result<()> {
        let slice = match &mut self.open {
            Some(slice) if slice.number == number => slice,
            _ => self.open(number)?,
        };
        if offset as u64 > slice.size {
            return format_err(format!("offset {offset} lies outside slice {}", number + 1));
        }
        slice.file.seek(SeekFrom::Start(offset as u64))?;
        slice.pos = offset as u64;
        Ok(())
    }
}

impl Read for SliceReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let slice = match &mut self.open {
            None => return Err(io::Error::other("no slice is open")),
            // At the end of a slice the bytes go on in the next one, after its head.
            Some(slice) if slice.pos >= slice.size => {
                let next = slice.number + 1;
                self.open(next).map_err(io::Error::other)?
            }
            Some(slice) => slice,
        };
        let want = out.len().min((slice.size - slice.pos) as usize);
        let got = slice.file.read(&mut out[..want])?;
        if got == 0 && want > 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "slice file is shorter than its head says"));
        }
        slice.pos += got as u64;
        Ok(got)
    }
}

/// The packed bytes of one chunk: `left` bytes from where the slices stand.
struct ChunkReader<'a> {
    slices: &'a mut SliceReader,
    left: u64,
}

impl Read for ChunkReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let want = out.len().min(self.left.min(usize::MAX as u64) as usize);
        if want == 0 {
            return Ok(0);
        }
        let got = self.slices.read(&mut out[..want])?;
        self.left -= got as u64;
        Ok(got)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The call-instruction filter
// ---------------------------------------------------------------------------------------------------------------

/// Undoes Inno Setup's "call instruction optimisation" (the form of versions 5.3.9 and later) on a program file.
///
/// The packager rewrites the 4-byte target of every x86 `call` (`E8`) and `jmp` (`E9`) from "relative to the next
/// instruction" to "absolute in the file", because the same target then gives the same bytes wherever it is
/// called from, which packs better. Only targets whose top byte is `00` or `FF` are touched, only the low three
/// bytes are changed (the top byte is flipped if the relative target's bit 23 is set, so that it is `00` for calls
/// in both directions), and an instruction that would straddle a 64 KiB boundary is left alone. This turns the
/// bytes back.
#[derive(Debug, Clone, Default)]
pub struct CallFilter {
    /// Bytes of the file gone through so far.
    offset: u32,
}

impl CallFilter {
    pub fn new() -> Self {
        CallFilter::default()
    }

    /// Decodes the next bytes of the file in place and returns how many of them are finished. Unless `last` is
    /// set, an instruction cut off by the end of `data` is left for the next call: the caller passes the
    /// unfinished tail (at most 4 bytes) again, at the front of the next piece.
    pub fn decode(&mut self, data: &mut [u8], last: bool) -> usize {
        let mut i = 0;
        while i < data.len() {
            let at = self.offset.wrapping_add(i as u32);
            let byte = data[i];
            i += 1;
            if byte != 0xe8 && byte != 0xe9 {
                continue;
            }
            if 0x10000 - (at % 0x10000) < 5 {
                continue;
            }
            if i + 4 > data.len() {
                if last {
                    // A cut-off instruction at the very end of the file stays as it is.
                    i = data.len();
                } else {
                    i -= 1;
                }
                break;
            }
            let target = &mut data[i..i + 4];
            if target[3] == 0x00 || target[3] == 0xff {
                // The address of the next instruction, as the packager counted it: 24 bits.
                let next = at.wrapping_add(5) & 0x00ff_ffff;
                let absolute = u32::from_le_bytes([target[0], target[1], target[2], 0]);
                let relative = absolute.wrapping_sub(next);
                target[0] = relative as u8;
                target[1] = (relative >> 8) as u8;
                target[2] = (relative >> 16) as u8;
                if relative & 0x0080_0000 != 0 {
                    target[3] = !target[3];
                }
            }
            i += 4;
        }
        self.offset = self.offset.wrapping_add(i as u32);
        i
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Tests with made-up bytes (tests against the real package are in tests/inno.rs)
// ---------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destinations_are_taken_apart() {
        let d = Destination::parse(r"{app}\cd_image\data\x.dat");
        assert_eq!(d.root, Root::App);
        assert_eq!(d.path, "cd_image/data/x.dat");
        assert_eq!(d.app_relative(), Some("cd_image/data/x.dat"));

        let d = Destination::parse(r"{sys}\some.dll");
        assert_eq!(d.root, Root::Other("sys".into()));
        assert_eq!(d.app_relative(), None);

        let d = Destination::parse(r"{code:Where|{app}}\a.txt");
        assert_eq!(d.root, Root::Other("code:Where|{app}".into()));
        assert_eq!(d.path, "a.txt");

        let d = Destination::parse(r"{app}\{code:Sub}\a.txt");
        assert!(d.has_inner_constant);
        assert_eq!(d.app_relative(), None);

        assert_eq!(Destination::parse(r"{app}\..\evil.txt").app_relative(), None);
        assert_eq!(Destination::parse(r"{app}\c:\evil.txt").app_relative(), None);
        assert_eq!(Destination::parse(r"{app}").app_relative(), None);
        assert_eq!(Destination::parse(r"C:\x.txt").root, Root::None);
        assert_eq!(Destination::parse("").root, Root::None);
    }

    #[test]
    fn folders_get_one_spelling() {
        let mut plan: Vec<PlannedFile> = ["Data/Gui/a.txt", "data/GUI/B.txt", "data/other/c.txt", "d.txt"]
            .iter()
            .map(|p| PlannedFile { file: 0, path: p.to_string() })
            .collect();
        unify_folder_case(&mut plan);
        let paths: Vec<&str> = plan.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(paths, ["Data/Gui/a.txt", "Data/Gui/B.txt", "Data/other/c.txt", "d.txt"]);
    }

    #[test]
    fn flag_sets_read_as_delphi_stores_them() {
        // 9 flags: two bytes. 17 flags: three bytes, padded to four. 44 flags: six bytes.
        let bytes = [0x01, 0x01, 0xaa, 0x02, 0x00, 0x01, 0xff, 0xbb, 1, 2, 3, 4, 5, 6, 0xcc];
        let mut c = Cursor::new(&bytes);
        assert_eq!(c.flags(9).unwrap(), 0x0101);
        assert_eq!(c.u8().unwrap(), 0xaa);
        assert_eq!(c.flags(17).unwrap(), 0x01_0002);
        assert_eq!(c.u8().unwrap(), 0xbb);
        assert_eq!(c.flags(44).unwrap(), 0x0605_0403_0201);
        assert_eq!(c.u8().unwrap(), 0xcc);
        assert!(c.u8().is_err());
        assert_eq!(header_flags::COUNT, 44);
        assert_eq!(file_flags::COUNT, 32);
        assert_eq!(file_flags::DONT_COPY, 1 << 19);
        assert_eq!(data_flags::CALL_INSTRUCTION_OPTIMIZED, 1 << 4);
        assert_eq!(data_flags::names(0b1000_0100), ["TIME_STAMP_IN_UTC", "CHUNK_COMPRESSED"]);
    }

    #[test]
    fn expressions() {
        let set = |name: &str| matches!(name, "main" | "extra\\help");
        assert_eq!(eval_expression("", &set), Some(true));
        assert_eq!(eval_expression("main", &set), Some(true));
        assert_eq!(eval_expression("other", &set), Some(false));
        assert_eq!(eval_expression("other main", &set), Some(true));
        assert_eq!(eval_expression("main and not other", &set), Some(true));
        assert_eq!(eval_expression("not (main or other)", &set), Some(false));
        assert_eq!(eval_expression("(other another) or main", &set), Some(true));
        assert_eq!(eval_expression("extra\\help and other", &set), Some(false));
        assert_eq!(eval_expression("(main", &set), None);
        assert_eq!(eval_expression("main)", &set), None);
        assert_eq!(eval_expression("and", &set), None);
    }

    #[test]
    fn slice_names() {
        assert_eq!(slice_name("install", 0, 1), "install-1.bin");
        assert_eq!(slice_name("install", 4, 1), "install-5.bin");
        assert_eq!(slice_name("setup", 3, 2), "setup-2b.bin");
    }

    #[test]
    fn code_page_1252() {
        assert_eq!(decode_windows_1252(b"caf\xe9 \x99 \x80"), "caf\u{e9} \u{2122} \u{20ac}");
    }

    #[test]
    fn times() {
        let mut d = DataEntry { filetime: FILETIME_UNIX_EPOCH + 15_000_000, ..DataEntry::default() };
        assert_eq!(d.unix_time(), 1);
        assert_eq!(d.modified(), Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1500)));
        d.filetime = 0;
        assert_eq!(d.modified(), None);
    }

    #[test]
    fn not_an_installer() {
        assert!(matches!(Package::parse(b"MZ not a program", PathBuf::new(), "x".into()), Err(Error::Format(_))));
        assert!(matches!(Package::parse(&[], PathBuf::new(), "x".into()), Err(Error::Format(_))));
    }

    #[test]
    fn offset_table_is_checked() {
        // An old-style pointer at 0x30 to a table at 0x40.
        let mut exe = vec![0u8; 0x40 + 44];
        exe[0x30..0x34].copy_from_slice(b"Inno");
        exe[0x34..0x38].copy_from_slice(&0x40u32.to_le_bytes());
        exe[0x38..0x3c].copy_from_slice(&(!0x40u32).to_le_bytes());
        exe[0x40..0x4c].copy_from_slice(&OFFSET_TABLE_MAGIC);
        exe[0x4c..0x50].copy_from_slice(&1u32.to_le_bytes());
        exe[0x60..0x64].copy_from_slice(&0x1234u32.to_le_bytes());
        let crc = crc32(&exe[0x40..0x40 + 40]);
        exe[0x68..0x6c].copy_from_slice(&crc.to_le_bytes());
        let table = read_offset_table(&exe).unwrap();
        assert_eq!((table.table_offset, table.header_offset, table.data_offset), (0x40, 0x1234, 0));
        exe[0x61] ^= 1;
        assert!(matches!(read_offset_table(&exe), Err(Error::Checksum { .. })));
    }

    /// A block as the setup data stores it: head, then pieces of 4096 each led by its CRC-32.
    fn block(body: &[u8], packed: bool) -> Vec<u8> {
        let mut pieces = Vec::new();
        for piece in body.chunks(4096) {
            pieces.extend_from_slice(&crc32(piece).to_le_bytes());
            pieces.extend_from_slice(piece);
        }
        let mut head = (pieces.len() as u32).to_le_bytes().to_vec();
        head.push(packed as u8);
        let mut out = crc32(&head).to_le_bytes().to_vec();
        out.extend_from_slice(&head);
        out.extend_from_slice(&pieces);
        out
    }

    #[test]
    fn blocks_are_joined_and_checked() {
        let body: Vec<u8> = (0..10_000u32).map(|i| (i * 7) as u8).collect();
        let mut bytes = vec![0xee; 3];
        bytes.extend_from_slice(&block(&body, false));
        bytes.push(0x55);
        let mut pos = 3;
        assert_eq!(read_block(&bytes, &mut pos).unwrap(), body);
        assert_eq!(pos, bytes.len() - 1);
        // One flipped bit in the second piece.
        bytes[3 + 9 + 4100 + 50] ^= 4;
        let mut pos = 3;
        assert!(matches!(read_block(&bytes, &mut pos), Err(Error::Checksum { .. })));
        // A flipped bit in the head.
        let mut bytes = block(&body, false);
        bytes[5] ^= 1;
        assert!(matches!(read_block(&bytes, &mut 0), Err(Error::Checksum { .. })));
    }

    fn put_text(out: &mut Vec<u8>, text: &str) {
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(text.as_bytes());
    }

    /// One made-up file of a made-up package.
    struct Made {
        destination: &'static str,
        content: Vec<u8>,
        program: bool,
    }

    /// Writes a small package into a new folder: `install.exe` with stored (unpacked) setup data and the files
    /// stored in one chunk, cut into slices of `slice_payload` bytes. Program files go in with their calls made
    /// absolute, as the packager stores them.
    fn made_up_package(name: &str, files: &[Made], slice_payload: usize) -> PathBuf {
        let mut first = Vec::new();
        for i in 0..30 {
            // The 29 texts of the header and its script: only the name and the version are filled in.
            put_text(&mut first, [(0, "Made-up"), (9, "1.2.3")].iter().find(|(n, _)| *n == i).map_or("", |f| f.1));
        }
        first.extend_from_slice(&[0; 32]);
        for i in 0..16u32 {
            // The files and their data entries; no other entries.
            first.extend_from_slice(&(if i == 7 || i == 8 { files.len() as u32 } else { 0 }).to_le_bytes());
        }
        first.extend_from_slice(&[0; 20 + 12 + 20 + 8 + 8]);
        first.extend_from_slice(&1u32.to_le_bytes()); // slices per disk
        first.extend_from_slice(&[0, 0, 0, 0, 0, 3, 0, 0, 0, 0]); // … compression LZMA1 (the chunk says "stored") …
        first.extend_from_slice(&[0; 4 + 6]);
        let mut second = Vec::new();
        let mut chunk = Vec::new();
        for (i, file) in files.iter().enumerate() {
            put_text(&mut first, "");
            put_text(&mut first, file.destination);
            for _ in 0..8 {
                put_text(&mut first, "");
            }
            first.extend_from_slice(&[0; 20]);
            first.extend_from_slice(&(i as u32).to_le_bytes());
            first.extend_from_slice(&[0; 4 + 8]);
            first.extend_from_slice(&(-1i16).to_le_bytes());
            first.extend_from_slice(&(file_flags::IGNORE_VERSION | file_flags::CREATE_ALL_SUB_DIRS).to_le_bytes());
            first.push(0);

            chunk.extend_from_slice(b"gap"); // (files need not lie back to back)
            let mut sum = Sha1::new();
            sum.update(&file.content);
            let mut stored = file.content.clone();
            if file.program {
                encode_calls(&mut stored);
            }
            second.extend_from_slice(&[0; 8]); // slices: filled in below
            second.extend_from_slice(&12u32.to_le_bytes());
            second.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
            second.extend_from_slice(&(stored.len() as u64).to_le_bytes());
            second.extend_from_slice(&[0; 8]); // chunk size: filled in below
            second.extend_from_slice(&sum.finish());
            second.extend_from_slice(&(FILETIME_UNIX_EPOCH + 1_000_000_000 * 10_000_000).to_le_bytes());
            second.extend_from_slice(&[0; 8]);
            let flags = if file.program { data_flags::CALL_INSTRUCTION_OPTIMIZED } else { 0 };
            second.extend_from_slice(&flags.to_le_bytes());
            chunk.extend_from_slice(&stored);
        }
        put_text(&mut first, "");
        put_text(&mut first, "");

        let mut packed = CHUNK_MAGIC.to_vec();
        packed.extend_from_slice(&chunk);
        let slices: Vec<&[u8]> = packed.chunks(slice_payload).collect();
        for entry in second.chunks_mut(74) {
            entry[4..8].copy_from_slice(&(slices.len() as u32 - 1).to_le_bytes());
            entry[28..36].copy_from_slice(&(chunk.len() as u64).to_le_bytes());
        }

        let mut exe = vec![0u8; 0x40];
        exe[0x30..0x34].copy_from_slice(b"Inno");
        exe[0x34..0x38].copy_from_slice(&0x40u32.to_le_bytes());
        exe[0x38..0x3c].copy_from_slice(&(!0x40u32).to_le_bytes());
        exe.extend_from_slice(&OFFSET_TABLE_MAGIC);
        exe.extend_from_slice(&1u32.to_le_bytes());
        exe.extend_from_slice(&[0; 16]);
        exe.extend_from_slice(&(0x40u32 + 44).to_le_bytes());
        exe.extend_from_slice(&0u32.to_le_bytes());
        let crc = crc32(&exe[0x40..]);
        exe.extend_from_slice(&crc.to_le_bytes());
        let mut line = VERSION_LINE.as_bytes().to_vec();
        line.resize(64, 0);
        exe.extend_from_slice(&line);
        exe.extend_from_slice(&block(&first, false));
        exe.extend_from_slice(&block(&second, false));

        let dir = std::env::temp_dir().join(format!("mao-install-inno-unit-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("install.exe"), exe).unwrap();
        for (i, payload) in slices.iter().enumerate() {
            let mut slice = SLICE_MAGIC.to_vec();
            slice.extend_from_slice(&(12 + payload.len() as u32).to_le_bytes());
            slice.extend_from_slice(payload);
            // (one name in upper case: a slice is found whatever the case of its name)
            let name = if i == 1 { "INSTALL-2.BIN".to_string() } else { format!("install-{}.bin", i + 1) };
            fs::write(dir.join(name), slice).unwrap();
        }
        dir
    }

    fn made_up_files() -> Vec<Made> {
        let mut code = vec![0x90u8; 3000];
        for (i, b) in code.iter_mut().enumerate() {
            if i % 11 == 0 {
                *b = 0xe8;
            } else if i % 11 == 4 {
                *b = if i % 3 == 0 { 0xff } else { 0x00 };
            } else if i % 7 == 0 {
                *b = i as u8;
            }
        }
        let big: Vec<u8> = (0..5000u32).map(|i| (i / 3) as u8).collect();
        vec![
            Made { destination: r"{app}\readme.txt", content: b"made-up text\r\n".to_vec(), program: false },
            Made { destination: r"{app}\Data\tool.dll", content: code, program: true },
            Made { destination: r"{app}\data\Empty.bin", content: Vec::new(), program: false },
            Made { destination: r"{app}\data\big.bin", content: big, program: false },
            Made { destination: r"{sys}\elsewhere.dll", content: b"not for the game folder".to_vec(), program: false },
            Made { destination: r"{app}\..\outside.txt", content: b"must not be written".to_vec(), program: false },
        ]
    }

    #[test]
    fn made_up_package_unpacks_across_slices() {
        let files = made_up_files();
        let dir = made_up_package("whole", &files, 1000);
        let package = Package::open(&dir).unwrap();
        assert_eq!(package.header().app_name, "Made-up");
        assert_eq!(package.header().app_version, "1.2.3");
        assert_eq!(package.files().len(), files.len());
        assert!(package.slice_names().len() > 5);

        // Only the files that go below {app}, with one spelling for the folder.
        let plan = package.plan_app();
        let paths: Vec<&str> = plan.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["readme.txt", "Data/tool.dll", "Data/Empty.bin", "Data/big.bin"]);

        let out = dir.join("out");
        let mut seen = Vec::new();
        let done = package.extract(&out, &plan, &mut |p| {
            seen.push((p.bytes_done, p.files_done));
            true
        }).unwrap();
        let total: u64 = files[..4].iter().map(|f| f.content.len() as u64).sum();
        assert_eq!(done, Extracted { files: 4, bytes: total });
        assert_eq!(package.plan_size(&plan), total);
        assert_eq!(seen.last(), Some(&(total, 3)));
        assert!(seen.windows(2).all(|w| w[0] <= w[1]));
        for (file, planned) in files.iter().zip(&plan) {
            assert_eq!(fs::read(out.join(&planned.path)).unwrap(), file.content, "{}", planned.path);
            let modified = fs::metadata(out.join(&planned.path)).unwrap().modified().unwrap();
            assert_eq!(modified, SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000));
        }
        assert!(!dir.join("outside.txt").exists());

        // A chosen plan can take a file that goes elsewhere, under any safe name…
        let elsewhere = Root::Other("sys".into());
        let plan = package.plan(|_, target| (target.root == elsewhere).then(|| "sys/e.dll".to_string()));
        assert_eq!(package.extract(&out, &plan, &mut |_| true).unwrap().files, 1);
        assert_eq!(fs::read(out.join("sys/e.dll")).unwrap(), files[4].content);
        // …but not under a name that leaves the folder.
        let escape = [PlannedFile { file: 0, path: "../x.txt".into() }];
        assert!(matches!(package.extract(&out, &escape, &mut |_| true), Err(Error::Format(_))));
        // Two entries for one path: the later one is written.
        let twice =
            [PlannedFile { file: 0, path: "same.bin".into() }, PlannedFile { file: 3, path: "Same.bin".into() }];
        assert_eq!(package.extract(&out, &twice, &mut |_| true).unwrap().files, 1);
        assert_eq!(fs::read(out.join("Same.bin")).unwrap(), files[3].content);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A progress function that answers `false` stops the unpacking: the file it was at is not left, under its
    /// name or its temporary one, and the files before it are whole.
    #[test]
    fn a_stopped_unpacking_leaves_no_half_file() {
        let files = made_up_files();
        let dir = made_up_package("stopped", &files, 1000);
        let package = Package::open(&dir).unwrap();
        let plan = package.plan_app();
        let out = dir.join("out");
        let result = package.extract(&out, &plan, &mut |p| p.current != "Data/big.bin");
        assert!(matches!(result, Err(Error::Stopped)), "{result:?}");
        assert_eq!(fs::read(out.join("readme.txt")).unwrap(), files[0].content);
        assert_eq!(fs::read(out.join("Data/tool.dll")).unwrap(), files[1].content);
        assert!(!out.join("Data/big.bin").exists());
        assert!(!out.join("Data/big.bin.mao-part").exists());
        // (and the next time it unpacks whole)
        assert_eq!(package.extract(&out, &plan, &mut |_| true).unwrap().files, 4);
        assert_eq!(fs::read(out.join("Data/big.bin")).unwrap(), files[3].content);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_and_missing_slices_are_reported() {
        let files = made_up_files();
        let dir = made_up_package("damaged", &files, 1000);
        let package = Package::open(&dir).unwrap();
        let plan = package.plan_app();
        // One bit wrong in the last wanted file's bytes: its check sum fails and no file of that name is left.
        let last = package.data_of(&package.files()[3]).unwrap();
        let at = 4 + last.file_offset as usize + 100;
        let slice = dir.join(slice_name("install", (at / 1000) as u32, 1));
        let mut bytes = fs::read(&slice).unwrap();
        bytes[12 + at % 1000] ^= 0x10;
        fs::write(&slice, &bytes).unwrap();
        let out = dir.join("out");
        match package.extract(&out, &plan, &mut |_| true) {
            Err(Error::Checksum { what, .. }) => assert_eq!(what, "Data/big.bin"),
            other => panic!("{other:?}"),
        }
        assert!(out.join("Data/tool.dll").exists());
        assert!(!out.join("Data/big.bin").exists());
        assert!(!out.join("Data/big.bin.mao-part").exists());

        // A slice that is not there.
        fs::remove_file(dir.join("install-4.bin")).unwrap();
        match package.extract(&out, &plan, &mut |_| true) {
            Err(Error::MissingFile(name)) => assert!(name.ends_with("install-4.bin"), "{name}"),
            other => panic!("{other:?}"),
        }
        // A slice cut short.
        fs::write(dir.join("install-4.bin"), &bytes[..500]).unwrap();
        assert!(matches!(package.extract(&out, &plan, &mut |_| true), Err(Error::Format(_))));
        // No installer program at all.
        assert!(matches!(Package::open(dir.join("out")), Err(Error::MissingFile(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_versions_and_methods_are_refused() {
        let files = made_up_files();
        let dir = made_up_package("refused", &files, 100_000);
        let exe = fs::read(dir.join("install.exe")).unwrap();
        // Another version line.
        let mut other = exe.clone();
        other[0x6c + 23..0x6c + 26].copy_from_slice(b"5.5");
        assert!(matches!(Package::parse(&other, dir.clone(), "install".into()), Err(Error::Unsupported(_))));
        // Chunks packed with a method that is not unpacked here, and an encrypted one.
        let cases = [
            (Compression::Zlib, 0),
            (Compression::BZip2, 0),
            (Compression::Lzma2, 0),
            (Compression::Lzma1, data_flags::CHUNK_ENCRYPTED),
        ];
        for (method, flag) in cases {
            let mut package = Package::parse(&exe, dir.clone(), "install".into()).unwrap();
            package.header.compression = Some(method);
            for data in &mut package.data_entries {
                data.flags |= data_flags::CHUNK_COMPRESSED | flag;
            }
            let plan = package.plan_app();
            let result = package.extract(dir.join("out"), &plan, &mut |_| true);
            assert!(matches!(result, Err(Error::Unsupported(_))), "{method:?}: {result:?}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// The packager's side of the filter, written from its description: relative targets made absolute.
    fn encode_calls(data: &mut [u8]) {
        let mut i = 0;
        while i < data.len() {
            let byte = data[i];
            i += 1;
            if (byte != 0xe8 && byte != 0xe9) || 0x10000 - ((i - 1) % 0x10000) < 5 {
                continue;
            }
            if i + 4 > data.len() {
                break;
            }
            let t = &mut data[i..i + 4];
            if t[3] == 0x00 || t[3] == 0xff {
                let next = (i as u32 + 4) & 0x00ff_ffff;
                let relative = u32::from_le_bytes([t[0], t[1], t[2], 0]);
                if relative & 0x0080_0000 != 0 {
                    t[3] = !t[3];
                }
                let absolute = relative.wrapping_add(next);
                t[0] = absolute as u8;
                t[1] = (absolute >> 8) as u8;
                t[2] = (absolute >> 16) as u8;
            }
            i += 4;
        }
    }

    #[test]
    fn call_filter_known_bytes() {
        // A call at offset 2 whose packed target is 0x000107: the next instruction is at 7, so the relative
        // target is 0x000100. A jump at offset 9 with packed target 0x000004 gives -10: low bytes f6 ff ff and the
        // top byte flipped from 00 to ff. A call with top byte 0x12 is left alone.
        let mut data = [
            0x90, 0x90, 0xe8, 0x07, 0x01, 0x00, 0x00, 0x90, 0x90, 0xe9, 0x04, 0x00, 0x00, 0x00, 0xe8, 1, 2, 3, 0x12,
        ];
        let mut filter = CallFilter::new();
        assert_eq!(filter.decode(&mut data, true), data.len());
        assert_eq!(
            data,
            [0x90, 0x90, 0xe8, 0x00, 0x01, 0x00, 0x00, 0x90, 0x90, 0xe9, 0xf6, 0xff, 0xff, 0xff, 0xe8, 1, 2, 3, 0x12]
        );
    }

    #[test]
    fn call_filter_in_pieces_matches_one_pass() {
        // Made-up "code": plenty of E8/E9 bytes, some near 64 KiB boundaries and at the very end.
        let mut state = 0x1234_5678u32;
        let mut plain = vec![0u8; 200_000];
        for b in plain.iter_mut() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = match state >> 28 {
                0 => 0xe8,
                1 => 0xe9,
                2 | 3 => 0x00,
                4 => 0xff,
                _ => (state >> 16) as u8,
            };
        }
        let n = plain.len();
        // A call 1 byte before a 64 KiB boundary, one 4 bytes before the next, and one cut off by the file's end,
        // each with quiet bytes around it so nothing else reaches into them.
        plain[0xfff0..0x10008].fill(0x11);
        plain[0xffff] = 0xe8;
        plain[0x1fff0..0x20008].fill(0x11);
        plain[0x1fffc] = 0xe9;
        plain[n - 16..].fill(0x11);
        plain[n - 3] = 0xe8;

        let mut whole = plain.clone();
        assert_eq!(CallFilter::new().decode(&mut whole, true), n);
        assert_ne!(whole, plain);
        assert_eq!(whole[0xfff0..0x10008], plain[0xfff0..0x10008]);
        assert_eq!(whole[0x1fff0..0x20008], plain[0x1fff0..0x20008]);
        assert_eq!(whole[n - 16..], plain[n - 16..]);

        // Fed in pieces of every awkward size, with the unfinished tail carried over, the result is the same.
        for piece in [1usize, 2, 3, 5, 7, 64, 4096, 65_536, 70_001] {
            let mut filter = CallFilter::new();
            let mut out = Vec::with_capacity(n);
            let mut buffer: Vec<u8> = Vec::new();
            let mut at = 0;
            while at < n || !buffer.is_empty() {
                let end = (at + piece).min(n);
                buffer.extend_from_slice(&plain[at..end]);
                at = end;
                let ready = filter.decode(&mut buffer, at == n);
                assert!(buffer.len() - ready <= 4);
                out.extend_from_slice(&buffer[..ready]);
                buffer.drain(..ready);
            }
            assert_eq!(out, whole, "pieces of {piece}");
        }

        // And it undoes the packager's transformation.
        let original = whole;
        let mut packed = original.clone();
        encode_calls(&mut packed);
        assert_ne!(packed, original);
        CallFilter::new().decode(&mut packed, true);
        assert_eq!(packed, original);
    }
}
