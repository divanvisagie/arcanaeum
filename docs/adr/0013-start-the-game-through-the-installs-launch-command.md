# 0013. Start the game through the install's launch command

Date: 2026-11-07

## Status

Accepted

## Context

The GOG copy runs under umu and GE-Proton, inside a network namespace with only a loopback
interface, so it stays offline. A script, `bin/skyrim-gog`, does that and takes `--vanilla`
(`SkyrimSE.exe` without SKSE) and `--launcher` (Bethesda's launcher); with no argument it starts
SKSE. Starting the game meant leaving Arcanaeum for a desktop entry, after switching plugins
here.

Arcanaeum could run umu itself, but then it would own the Proton version, the prefix, the
offline namespace and the Bethesda protonfix, which are all choices the script already makes
and keeps in one place. Steam's copy has no equivalent: Steam can't be told which exe to run
without changing its launch options.

## Decision

On Linux only, each install gets an optional launch command (`launcher` in its `[[installs]]`
entry), a script taking `--vanilla` and `--launcher` as above. A Play menu in the title bar starts
SKSE, the game without SKSE, or Bethesda's launcher through it, in a process group of its own so
closing Arcanaeum doesn't end the game. With none set, it looks for a `skyrim-gog` in the `bin`
folder beside the game's folder, then on `PATH` (where the GOG setup's `install.sh` links it);
if it finds none the items are disabled.

Two settings add to the environment the command runs with: **Show FPS counter** sets
`DXVK_HUD=fps` (Proton draws through DXVK), and **Game flags** takes any `KEY=VALUE` words, for
`MANGOHUD=1` or `DXVK_HUD=full`. A `DXVK_HUD` in the flags wins over the checkbox. Flags that
aren't `KEY=VALUE` stop Settings from saving.

The code is `src/launch.rs`, compiled only for Linux, as are the menu and Settings' rows. The
config keys are plain data in every build so a config moved between machines keeps them.

Starting is refused while Skyrim is running, and while switched plugins wait for Apply
(ADR-0006): the game rewrites `Plugins.txt` as it quits, and would load the old order.

## Consequences

- A GOG copy starts offline from the app with the FPS counter on or off, with no change to
  `bin/skyrim-gog`.
- Steam's copy can use the same menu only by pointing it at a script of the user's own.
- The launch command's contract (`--vanilla`, `--launcher`) is Arcanaeum's, not a standard; a
  different script needs to follow it.
- The counter only shows if the game draws through DXVK; ENB's wrapper `d3d11.dll` sits on top
  of it and may hide or move it.
- Windows has no Play menu yet.
