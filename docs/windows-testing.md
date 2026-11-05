# Testing the Windows build on Linux

Arcanaeum runs on Windows and Linux. To check the Windows side before merging, without a Windows
machine, build it for `x86_64-pc-windows-gnu` and run it as a Windows program under the Wine inside a
Proton build (GE-Proton, as used for the games). Nothing here is installed system-wide or committed:
the toolchain and the Windows prefix live in `target/windows`.

```bash
make windows-setup   # once: the mingw toolchain (no root) and the Windows prefix
make windows-test    # the tests, as Windows binaries under Wine
make windows-run     # arcanaeum.exe itself, in the throwaway prefix
make windows-clean   # delete target/windows
```

`scripts/windows.sh doctor` says what is missing. It needs:

- the Rust target: `rustup target add x86_64-pc-windows-gnu`
- mingw-w64 with POSIX threads (`sudo apt install gcc-mingw-w64-x86-64-posix`). On Debian and Ubuntu
  `make windows-setup` fetches the packages and unpacks them under `target/windows/mingw` instead.
  Elsewhere install mingw-w64 with your package manager.
- a Wine: a GE-Proton folder (found under Steam's `compatibilitytools.d`, or set `PROTONPATH`), or a
  `wine` on PATH.

Where things are can be set in the environment or in `scripts/windows.local.sh` (ignored by git):

```bash
export PROTONPATH=/path/to/GE-Proton11-10-x86_64
# export MINGW_BIN=/path/with/x86_64-w64-mingw32-gcc
# export WINEPREFIX=...   # never a real game's prefix
```

## What it tells you

`windows-test` runs the same tests as `make test`, compiled for Windows, so the `cfg(windows)` code
and Windows path handling are used for real: `C:\` and `Z:\` paths, `\` separators, the user's
`%APPDATA%` and `%LOCALAPPDATA%` folders (inside the prefix, as user `steamuser`). The tests write
config and `Plugins.txt` under that Windows user, which is why the prefix must be the throwaway one.

`windows-run` starts the real window. To see the app on a game, give the prefix a config and the
files it reads, which are copies or read-only sources:

```
target/windows/prefix/drive_c/users/steamuser/AppData/Roaming/arcanaeum/config.toml
    [[installs]]
    name = "Game"
    game_folder = 'Z:\path\to\Skyrim Special Edition'      # Z: is the Linux root
target/windows/prefix/drive_c/users/steamuser/AppData/Local/Skyrim Special Edition/Plugins.txt
```

(`Skyrim Special Edition GOG` for a GOG copy.) Without a `Plugins.txt` there, the load order is only
the base game. Screenshot mode works too: `ARCANAEUM_SAVE_FOLDER='Z:\…\Saves'
ARCANAEUM_SCREENSHOT='Z:\tmp\out.png' scripts/windows.sh run`.

## What it doesn't

It is Wine, not Windows. It doesn't exercise the real registry or Steam's, `tasklist` finding
SkyrimSE.exe, native file dialogs, or the MSVC build (this is the GNU target). Pass it, then still
look at the real thing before a release.

Skipped on purpose: the Nexus search test (needs the network) and two `features/` tests that look for
a real Skyrim install, which the prefix hasn't.

The same environment tests skyplug: from its checkout, with the variables `scripts/windows.sh` sets
(`CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER` and `…_RUNNER`), `cargo test --target x86_64-pc-windows-gnu`.
