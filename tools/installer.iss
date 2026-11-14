; The Windows installer (Inno Setup 6): the folder tools/package.ps1 makes (dist/ModernAnarchyOnline: mao-client.exe,
; assets, play.bat, README.txt, LICENSE) put on the machine with a Start menu entry, an optional desktop shortcut and an
; uninstaller. For one user by default, without an administrator's password; "for all users" can be chosen instead.
;
;   tools\package.ps1 -Version 0.2.0
;   iscc /DVersion=0.2.0 tools\installer.iss        -> dist\ModernAnarchyOnline-0.2.0-windows-x64-setup.exe
;
; For Windows on ARM64 the folder is made with package.ps1's -Target aarch64-pc-windows-msvc and the installer with
; /DArch=arm64 (-> ...-windows-arm64-setup.exe): that one installs on ARM64 machines only. The x64 one installs there
; too, and the client then runs under Windows' x64 emulation.
;
; The release workflow (.github/workflows/release.yml) does both; see docs/RELEASING.md. The installer is not signed:
; Windows SmartScreen asks before it runs ("More info", "Run anyway"). Nothing of the game is in it, and the client's
; settings (%APPDATA%\ModernAnarchyOnline) are left where they are when it is removed.
#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Arch
  #define Arch "x64"
#endif
#if Arch == "arm64"
  #define Machines "arm64"
#else
  #define Machines "x64compatible"
#endif
; (the numbers alone for the file's version resource: 0.2.0-rc.1 is 0.2.0 there)
#define Numbers Copy(Version, 1, Pos("-", Version + "-") - 1)

[Setup]
AppId={{22F826DC-AAD8-43F5-8A25-ADAFE896D881}
AppName=Modern Anarchy Online
AppVersion={#Version}
AppVerName=Modern Anarchy Online {#Version}
AppPublisherURL=https://github.com/dracul89/modern-anarchy-online
AppSupportURL=https://github.com/dracul89/modern-anarchy-online/issues
VersionInfoVersion={#Numbers}
DefaultDirName={autopf}\Modern Anarchy Online
DefaultGroupName=Modern Anarchy Online
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed={#Machines}
ArchitecturesInstallIn64BitMode={#Machines}
LicenseFile=..\LICENSE
SetupIconFile=..\assets\icon\mao.ico
UninstallDisplayIcon={app}\mao-client.exe
UninstallDisplayName=Modern Anarchy Online
OutputDir=..\dist
OutputBaseFilename=ModernAnarchyOnline-{#Version}-windows-{#Arch}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\dist\ModernAnarchyOnline\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
; (the client started alone is the launcher: the dimension, the game folder under SETTINGS, PLAY)
Name: "{group}\Modern Anarchy Online"; Filename: "{app}\mao-client.exe"; WorkingDir: "{app}"
Name: "{group}\Read me"; Filename: "{app}\README.txt"
Name: "{autodesktop}\Modern Anarchy Online"; Filename: "{app}\mao-client.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\mao-client.exe"; Description: "{cm:LaunchProgram,Modern Anarchy Online}"; Flags: nowait postinstall skipifsilent
