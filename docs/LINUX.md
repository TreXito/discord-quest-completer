# Linux port

Discord Quest Completer now builds and runs on Linux, and it supports **every
application in Discord's detectable list** (24,883 entries at the time of
writing), not just the seven that ship a native Linux executable name.

Two ways to use it:

1. **GUI** - the same Tauri/Vue app, plus a Linux-only runner.
2. **CLI** - `cli/dqc`, a single-file Python tool with no dependencies. No
   WebKit, no Tauri, no Rust toolchain. Use this on servers, on odd distros, or
   if you just want `dqc play <game>` from a terminal.

---

## Why it works on Linux (and why no game files are needed)

This was verified against the real client, not guessed. Discord 1.0.166, Linux
`tar.gz` from `discord.com/api/download?platform=linux`, unpacked, native module
inspected (`modules/discord_utils-1/discord_utils/discord_utils.node`) and web
bundle read (`resources/app.asar` + `https://discord.com/assets/web.*.js`).

1. The client downloads its detection database from
   `https://cdn.discordapp.com/detectables/games-v1.json`. That file contains
   **all** executables per application, including `win32` ones:
   `{"executables":[{"name":"overwatch.exe","os":"win32"}],"id":"356875221078245376","name":"Overwatch"}`.
2. The renderer hands *every* application's executable names to the native
   process observer via `setObservedGamesCallback2()` (see
   `NativeModule.setObservedGamesCallback` in the web bundle). There is no
   per-platform filter: the mapping function keeps `{os, name, arguments}` as-is.
3. `process_observer_linux.cpp` polls `/proc/<pid>/exe`, skips system and
   compatibility-layer paths (`/bin/`, `/sbin/`, `wine-preloader`,
   `steamapps/common/proton`, `.../steamlinuxruntime`, `.../steamworks shared`)
   and matches the executable name against those candidates
   (`discord::util::RunningProcess{exePath, exePathNormalized, exeName, cmdLine}`,
   `HashLowerCase()`, `TryMatchFromDB`).
4. A match becomes a `RunningGame`. Quest progress for `PLAY_ON_DESKTOP` quests is
   driven by `RunningGameStore.getRunningGames()`, which needs **no window** - the
   only things consulted are the blocklist (`/games/detectable/exclusions`) and the
   "non-game / streamer tool" list.

So on Linux the registration *is* the file name. The runner is copied to
`<app id>/<executable name from the list>` and executed; the process shows up in
`/proc` as `.../games/<app id>/overwatch.exe` and Discord matches it. No Proton,
no Wine, no 80 GB download, no window.

Verified locally by replaying the same algorithm (`dqc doctor` does this too):

```
$ dqc play overwatch
Overwatch: faking overwatch.exe (pid 463128)
  exe  ~/.local/share/discord-quest-completer/games/356875221078245376/overwatch.exe

$ dqc doctor
  pid 463128: ~/.local/share/discord-quest-completer/games/356875221078245376/overwatch.exe -> Overwatch
```

Nested names work the same way, because the registered name is a path suffix
(`game/eldenring.exe`, `bin/x64/game.exe`): the tool recreates the folder
structure, exactly like the Windows runner does.

---

## GUI

### Build

System dependencies (Debian/Ubuntu names):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
                 librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
                 build-essential curl file
```

Fedora/Nobara:

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
                 librsvg2-devel patchelf libsoup3-devel javascriptcoregtk4.1-devel \
                 gcc gcc-c++ make
```

Then:

```bash
pnpm install
pnpm build:runner:linux      # builds src-linux (cargo, or gcc as a fallback)
pnpm copy:runner:linux       # -> src-tauri/resources/src-linux
pnpm tauri dev               # or: pnpm build:linux for a release bundle
```

`src-tauri/tauri.linux.conf.json` is picked up automatically by Tauri 2 and maps
`resources/src-linux` to `data/src-linux` next to the executable, which is where
`create_fake_game` looks for the runner template.

The app keeps its fake games in `games/` **next to the executable** (same as
Windows). Extract the AppImage or the tarball somewhere you can write to; a
read-only location such as `/usr/bin` will fail to create dummy files.

### Runner

`src-linux/src/main.c` is ~15 KB, links only libc, opens no window, and runs
until killed:

```
--title <name>      cosmetic, kept for parity with the Windows/macOS runners
--duration <secs>   exit after N seconds (dqc play --for 15m uses this)
<other args>        passed through, because a few detectable entries are only
                    matched together with their command line arguments
```

It sets `comm` (`PR_SET_NAME`) to the executable name so `ps`/`top` show the game,
ignores `SIGHUP` (it must outlive the launcher and the terminal) and exits on
`SIGTERM`/`SIGINT`, which is what `dqc stop` sends.

Build it standalone with either toolchain:

```bash
cd src-linux && ./build.sh           # cargo if usable, else cc
FORCE_CC=1 ./build.sh                # force the C compiler
./build.sh --copy                    # also copy into src-tauri/resources/
```

---

## CLI

```bash
./cli/dqc update                     # fetch the detection list + blocklist
./cli/dqc ls                         # first 25 applications and their exe
./cli/dqc ls terraria                # search by name, alias or executable
./cli/dqc info "elden ring"          # every registered executable + which one it would use
./cli/dqc play "elden ring"          # start faking it
./cli/dqc play overwatch --for 15m   # auto-stop after a 15 minute quest
./cli/dqc status                     # what is running, for how long
./cli/dqc stop --all                 # stop everything
./cli/dqc clean                      # delete folders of stopped fake games
./cli/dqc doctor                     # runner, database, Discord and what it can see
```

Files live in `${XDG_DATA_HOME:-~/.local/share}/discord-quest-completer`:

```
games-v1.json                 detection list snapshot
exclusions.json               Discord's blocklist
runner/src-linux              the runner, copied out of the repo on first use
games/<app id>/.dqc.json      manifest: app name, executable, arguments
games/<app id>/.dqc.pid       pid of the running fake game
games/<app id>/<exe path>     the actual dummy executable
```

Executable choice: a native `linux` name if one exists, otherwise the `win32`
name, preferring the game's own name over helpers (`worldofwarcraft.app` loses to
`_retail_/wow.exe`, `bin/gmad.exe` loses to `win64/gmod.exe`), skipping
Discord's blocklisted names and test/beta channel folders. Override with
`--exe <name>`.

---

## Quest types

| Quest | State |
| --- | --- |
| Play on desktop (`PLAY_ON_DESKTOP`) | works, this is what the fake game triggers |
| Watch a video (`WATCH_VIDEO`) | unrelated to fake games, Discord shows the video itself |
| Stream the game (`STREAM_ON_DESKTOP`) | not covered: Discord requires an actual stream of the game |
| Play an activity (`PLAY_ACTIVITY`) | not covered: that is a voice-channel activity |

Quests that only want playtime are covered. A quest that explicitly asks Discord
to launch the game will launch the dummy executable (the path exists), which then
runs as a normal fake game - untested here, since it depends on the quest's
launch configuration.

---

## Known limits

- **Fingerprint-verified applications.** Some entries in the list carry
  `hashes`/executable fingerprints and are matched harder than by name. A dummy
  executable cannot reproduce those. Symptom: the game never appears under
  Settings - Registered Games. Pick another executable of the same application,
  or accept it.
- **Blocklisted names.** Discord ignores generic executable names
  (`launcher.exe`, `install.exe`, `unitycrashhandler64.exe`, `vcredist*.exe`, ...).
  The tool warns when the only available name is one of those, because the fake
  game will not be detected.
- **Database freshness.** Discord only matches once it has downloaded its list,
  which happens ~1 minute after the client connects. If nothing shows up, wait a
  minute, then look at the activity panel - the tool and Discord do not share
  state, so the client is the only source of truth.
- **No window.** On macOS a visible window is needed; on Linux it is not. The
  fake process is invisible in the taskbar, which is intentional.
- **Flatpak/Snap Discord** sees `/proc` normally, so detection works, but the
  fake game must run as the same user as the Discord client.
- **Don't fake two games at once.** Discord tracks one "current game" for the
  activity panel and quests key off the running game list; run one at a time.
