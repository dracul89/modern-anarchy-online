Modern Anarchy Online
=====================

A new client for Anarchy Online. It is not the game: it reads the game's own files from an
installed copy of Anarchy Online (the folder that holds cd_image) and logs in to the game's
servers with your own account. Nothing of the game is in this package.

Starting it
-----------

Windows:      run mao-client.exe (the launcher: pick the dimension, set the game folder under
              SETTINGS if it was not found, press PLAY), or play.bat "<game folder>" to go
              straight to the login.
Linux, macOS: ./play.sh "<game folder>"   (or set MAO_GAME to the folder)

Keep mao-client and the assets folder together: the client looks for assets beside itself.

What it needs
-------------

- An installed Anarchy Online and an account for it.
- A graphics card with Direct3D 12 or Vulkan (Windows), Vulkan (Linux) or Metal (macOS).
- Windows: the WebView2 runtime for the launcher's news page and the in-game web pages
  (item store, daily rewards). Windows 11 has it; for Windows 10 it is Microsoft's
  "Evergreen WebView2 Runtime". Without it the client runs and those pages stay empty.
- Linux: ALSA (libasound2) for sound. The Linux and macOS builds are built by the same
  process as the Windows one but have seen little or no play: expect rough edges.
- macOS: the program is not signed. Clear the quarantine flag once:
      xattr -dr com.apple.quarantine ModernAnarchyOnline

Where it keeps its things
-------------------------

Settings, window places, hotbars, chat logs, screenshots and crash.log:
  Windows:      %APPDATA%\ModernAnarchyOnline
  Linux, macOS: ~/.config/ModernAnarchyOnline
Only the account's name is kept there, never the password.

Updating: unpack the new version over the old one, or anywhere else; the settings stay.

The client's code is under the MIT licence (LICENSE). The skins in assets/gui (AOU_*) are the
AOU GUI mod's and stay under that mod's own licence (GPL). Anarchy Online is Funcom's.
