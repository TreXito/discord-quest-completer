<p>
  <h1 align="center">Discord Quest Completer</h1>
</p>

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/markterence/discord-quest-completer/build-release.yaml?branch=main&style=flat&label=build%20(release))
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/markterence/discord-quest-completer/rust-check.yml?branch=main&style=flat&label=scheduled%20build%20%28every%202%20days%29)](https://github.com/markterence/discord-quest-completer/actions/workflows/rust-check.yml?query=branch%3Amain+event%3Aschedule)


> A quest completer for Discord. Discord Quest Completer. I don't know what to call this, but there it is.

A Windows desktop application for Discord Rich Presence and completing Discord Quest for games without needing to install the full actual games/applications. Perfect for completing Discord Quests and showing off your gaming status without the storage burden.


<!--
Old attachments: Kept here since github dont provide way to remove/manage attached files.

https://github.com/user-attachments/assets/34ff80c4-9e76-452c-9b02-e56f9ea706dd
https://github.com/user-attachments/assets/de904123-07df-41a9-8db1-ff05cc7ccc9b
-->

https://github.com/user-attachments/assets/de904123-07df-41a9-8db1-ff05cc7ccc9b

---

> [!NOTE]
> This repository is a fork of [markterence/discord-quest-completer](https://github.com/markterence/discord-quest-completer) (MIT) with a Linux port: the desktop app builds and runs on Linux, and a headless CLI is included. See [docs/LINUX.md](docs/LINUX.md) for how Discord's Linux detection actually works.

## ⚡ Quick start

**Linux, headless (nothing to compile, needs only `python3`):**

```bash
git clone https://github.com/TreXito/discord-quest-completer
cd discord-quest-completer
./cli/dqc update
./cli/dqc play "elden ring" --for 15m
./cli/dqc status
```

**Linux, GUI (compiles the Tauri app, ~3 minutes):**

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
                 librsvg2-devel patchelf libsoup3-devel javascriptcoregtk4.1-devel \
                 gcc gcc-c++ make pkgconf-pkg-config
corepack enable                     # provides pnpm, or: npm i -g pnpm
pnpm install
pnpm build:linux                    # runner + frontend + tauri release bundle
./src-tauri/target/release/discord-quest-completer
```

Full instructions, per-distro package names and troubleshooting: [Build from source](#-build-from-source).

---

## 📥 Installation

### Windows

You can download manually pre-built release binaries on the [Releases](https://github.com/markterence/discord-activity/releases) page.

Alternatively, you can follow the [build from source instructions](#-build-from-source) to build the app from source.

> [!IMPORTANT]
> Make sure you place or extract in a location where you have write or execute permissions.
> 
> The reason for this is that the app will create dummy game file in the same directory.
> By default, the app will not need to be run as administrator, unless if it was installed in a directory that requires elevated permissions. (e.g. `C:\Program Files\`, root of `C:\`, etc.)


> [!NOTE]
> Webview2 is required to run the app. WebView2 comes preinstalled on Windows 11. On versions older than Windows 11 you may need to install it manually.
> If you don't have it installed, you can download it from [here](https://developer.microsoft.com/en-us/microsoft-edge/webview2).

### 🐧 Linux

There are three ways in, pick one:

1. **CLI, no build at all.** `cli/dqc` is a single dependency-free Python file.
   It needs no WebKit, no Tauri, no Rust toolchain. This is the shortest path and
   it supports exactly the same applications as the GUI.

   ```bash
   ./cli/dqc update                  # download Discord's detection list
   ./cli/dqc ls                      # browse applications
   ./cli/dqc play "elden ring"       # start faking
   ./cli/dqc play overwatch --for 15m
   ./cli/dqc stop --all
   ```

2. **Pre-built release files.** The `publish` workflow builds a Linux tarball
   (`discord-quest-completer-linux-x86_64-<date>.tar.gz`) next to the Windows
   zip on the [Releases](../../releases) page. Extract it and run the binary - it
   expects the `data/` folder next to it, which the tarball already contains.

3. **Build it yourself** - see [Build from source](#-build-from-source).

On Linux the app writes its fake games to
`${XDG_DATA_HOME:-~/.local/share}/discord-quest-completer/games/`, not next to
the binary, so it also works from an AppImage, a `.deb` install or `/usr/bin`.
The CLI uses the same directory, so `dqc status` sees what the GUI started.

> [!IMPORTANT]
> Discord detects a game by the **file name** of a running process, so any of
> Discord's ~24,883 detectable applications works, not just the handful that
> publish a native Linux executable name. No Proton, no Wine, no game files.


## Uninstall

To uninstall the app, simply go to the folder where you extracted or placed the app and delete it's folder and thats it.

The content of the folder may look like this:

```text
discord-quest-completer/
├── discord-quest-completer.exe (main app)
├── data/ 
│   ├── src-win.exe (runner dummy template)
├── games/
│    ├── <game-id>/
```

On Linux the fake games live outside the app folder instead, so deleting the app
leaves them behind:

```text
~/.local/share/discord-quest-completer/
├── games/
│    ├── <app-id>/
│        ├── .dqc.json      (manifest: app name, executable, arguments)
│        ├── .dqc.pid       (pid while it runs)
│        └── overwatch.exe  (the dummy executable)
├── games-v1.json           (detection list snapshot, CLI only)
└── runner/src-linux        (runner copy, CLI only)
```

```bash
./cli/dqc stop --all && ./cli/dqc clean   # or simply: rm -rf ~/.local/share/discord-quest-completer
```

<!-- COMMENT:
  This folder on local app data seems to only exists when using the installer or when Web API like local storage was used? 
  Bring the section back once it's verified it was also existing using the quick portable builds.
-->
<!--
**Other optional files to remove when uninstalling:**

Delete the `me.markterence.discordquestcompleter` folder on `%localappdata%`. 

```bash
# Put this on File Explorer's address bar to navigate on the folder.
%localappdata%/me.markterence.discordquestcompleter
```

The folder `%localappdata%/me.markterence.discordquestcompleter` contains the WebView2 files used by Tauri for the this app. The contents of the folder varies per-user but it may look this this:

```
EBWebView
.cookies
```


> The `%localappdata%` is a system variable on Windows and is equivalent to `C:\Users\<YOUR USER>\AppData\Local\`.
-->

---

## ✨ Features

- Simulate playing verified Discord games without intalling a full game!
- Complete Discord Quests requiring 15-minute gameplay (not yet tested for Stream the game Quests)
- Only Discord Verified games are supported. The application fetches a list of games that Discord can automatically detect.
- Windows, Linux (GUI + CLI) and macOS. On Linux any of Discord's ~24,883 detectable applications can be faked, because the detection there is name based.

## ⚙️ How It Works

This app creates small executable files that mimic the actual game processes that Discord looks for when detecting a verified game to use it for it's Rich Presence activity.
When launched/played, the tiny executables trigger Discord's Rich Presence/Registed Games detection. Discord checks if Game exe name is running, sometimes it needs to be a folder where the game is supposed to be, thats how mainly it detects the Games, we can clearly see this on the "Registered Games" in the settings.

The dummy game executale files used by this program are placed in a folder called  `games/` folder relative to the main application's exe. As of release build v2025.10.07 the dummy executable file size is around 250kb, it could be smaller but it requires the end-users to install .NET Framework Runtime (which is sometimes comes pre-installed on an up-to-date Windows 11 PC's, so for now the dummy exe using WinAPI through C++ for compatibility rather than C#)

> [!TIP]
> After launching some games over a period of time, those files may accumulate. For a little maintenance, you can manually delete the created folders under the `games/` folder if you need to.


<!--
> _Currently, I am hesitant to add a file‑maintenance operation that deletes or clears the `games/` folder, because doing so may still cause unexpected issues. On Windows, the file system is case‑insensitive, so if you have a file named `Notes.txt` and you issue a command to delete `notes.txt`, Windows will still delete the `Notes.txt` file (the one that begins with a capital “N”)._
-->

## Use Cases

- Complete Discord Quests without downloading massive game files
- Show-off playing the latest games on your status if you want to. (Even if you don't really have it.) (LOL)
- Save disk space while still participating in Discord's Quest.
- I want to complete the Quest but I don't want to install the game's anti-cheat, game is too big, or it won't run on my PC.
- Useful for users with limited internet bandwidth or storage space

<!-- ## 🚀 Planned Features and fixes

- Make the "Stop" button work again if process was terminated outside of app's control.
- Persist games that added on the list so it wont reset.
- Discord Activity Simulator/playground (Customizable rich presence for developers and custom activities)
- Set custom activity status from supported games
- Linux and MacOS support (if possible) -->

## :heart: Support :heart:


[![GitHub][github-badge]][github-sponsors] - Become a Sponsor on GitHub. One time support, or a recurring donation

[![Paypal][paypal-badge]][paypal] - One-time donation via PayPal

<p align="center">
  <img src="https://hoshizora.markterence.me/github-stargazers/markterence/discord-quest-completer?show_usernames=1&v=1" align="center" alt="stargazers"/>
</p>

---

## 🖥️ Supported Platforms

- Windows 11 (not tested on Windows 10 but it should work)
- **Linux** (x86_64, GUI and headless CLI) - see [docs/LINUX.md](docs/LINUX.md)
- macOS (experimental, needs a visible window)

### 🐧 Linux

Linux works differently, and it works for **every** application Discord knows about
(~24,883), not just the few that ship a native Linux executable name.

Discord on Linux downloads `cdn.discordapp.com/detectables/games-v1.json`, hands
every executable name in it to `discord_utils.node`, and the Linux process
observer then walks `/proc/<pid>/exe` and matches the file name. There is no
platform filter, so a process named `overwatch.exe` is Overwatch. No window, no
Proton, no Wine.

Because of that the Linux runner (`src-linux/`) is ~15 KB of C that links only
libc: it is copied to `<app id>/<executable name>` and simply stays running.

```bash
pnpm install
pnpm build:runner:linux && pnpm copy:runner:linux   # or: cd src-linux && ./build.sh --copy
pnpm tauri dev                                      # GUI
```

Headless, no WebKit/Tauri needed at all:

```bash
./cli/dqc update                    # fetch Discord's detection list
./cli/dqc ls elden                 # search applications
./cli/dqc play "elden ring" --for 15m
./cli/dqc status
./cli/dqc stop --all
./cli/dqc doctor                    # check what Discord can currently see
```

Full mechanism notes, the CLI reference and the known limits (fingerprint-verified
apps, Discord's executable blocklist, quest types) live in
[docs/LINUX.md](docs/LINUX.md).

---

## 🛠️ Tech Stack

- 🦀 Rust
- 🌐 Vue.js
- 🧰 Tauri

## 🔨 Build from source

### 📋 Requirements

| Thing | Why |
| --- | --- |
| Rust (stable, 1.77+) | Tauri backend. `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node 20+ and pnpm | Vue frontend. `corepack enable` (ships with Node) or `npm i -g pnpm` |
| WebKitGTK 4.1 + GTK3 + libsoup3 | the window on Linux (Tauri 2, **not** the old 4.0) |
| gcc + make | builds the runner; the runner is plain C |

You do **not** need the WebKit stack if you only want the CLI: `./cli/dqc` is one
Python file with no dependencies.

### 🐧 Linux packages

**Nobara / Fedora:**

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
                 librsvg2-devel patchelf libsoup3-devel javascriptcoregtk4.1-devel \
                 openssl-devel gcc gcc-c++ make pkgconf-pkg-config
```

**Debian / Ubuntu / Mint:**

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
                 librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
                 build-essential curl file
```

**Arch:**

```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf libsoup3 base-devel
```

### 🛠️ Build

```bash
git clone https://github.com/TreXito/discord-quest-completer
cd discord-quest-completer
pnpm install
pnpm build:linux
```

`pnpm build:linux` is three steps, run them separately if something breaks:

```bash
pnpm build:runner:linux   # 1. compile src-linux (cargo, or gcc as a fallback)
pnpm copy:runner:linux    # 2. copy it to src-tauri/resources/src-linux
pnpm tauri build          # 3. frontend + Rust + bundling
```

The runner alone, without touching the app:

```bash
cd src-linux && ./build.sh            # cargo build --release, or cc if cargo is unusable
FORCE_CC=1 ./build.sh                 # force gcc/clang, ~30ms
./build.sh --copy                     # also install it into src-tauri/resources/
```

Output:

```text
src-tauri/target/release/discord-quest-completer        the app (needs data/src-linux next to it)
src-tauri/target/release/data/src-linux                 the runner template
src-tauri/target/release/bundle/deb/*.deb
src-tauri/target/release/bundle/appimage/*.AppImage
```

Run it straight from the build tree:

```bash
./src-tauri/target/release/discord-quest-completer
```

Live development (hot reload for the Vue side):

```bash
pnpm build:runner:linux && pnpm copy:runner:linux
pnpm tauri dev
```

### 🪟 Windows / 🍎 macOS

Same three steps, with the platform specific runner:

```bash
pnpm build:runner:win   && pnpm copy:runner:win     # Windows
pnpm build:runner:darwin && pnpm copy:runner:darwin # macOS
pnpm tauri dev          # or: pnpm tauri build
```

### 🧯 Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `Failed to resolve runner template` | you skipped the runner: `pnpm build:runner:linux && pnpm copy:runner:linux` |
| `webkit2gtk-4.1` not found, or `Package webkit2gtk-4.0 was not found` | wrong package generation; Tauri 2 wants **4.1** |
| Blank white window on NVIDIA / Wayland | `WEBKIT_DISABLE_DMABUF_RENDERER=1 ./src-tauri/target/release/discord-quest-completer` |
| AppImage will not start (`libfuse.so.2`) | `sudo dnf install fuse-libs`, or run it with `--appimage-extract-and-run` |
| Game never shows up in Discord | Discord needs its detection list (~1 min after login). Check with `./cli/dqc doctor`; some names are blocklisted by Discord and the tool warns about those |
| `dqc: no Linux runner found` | build the runner once: `cd src-linux && ./build.sh --copy`, or let it auto-pick `src-tauri/resources/src-linux` |

### 🗂️ Where the game list comes from

The app fetches it at runtime, best source first:

1. `https://cdn.discordapp.com/detectables/games-v1.json` - the file the Discord
   client itself uses (24,883 applications, every platform's executables).
2. the project's GitHub mirror,
3. `GET /api/v10/applications/detectable` - a smaller, older public mirror.

`src/assets/gamelist.json` is only a bundled fallback snapshot. `./cli/dqc update`
refreshes the CLI's own copy into `~/.local/share/discord-quest-completer/`.

## Other Thoughts

### The Discord's RPC server and Rich Presence 

There is also an experimental action like Discord RPC functionality along the selected game.
It connects to Discord's RPC Gateway to send Activity updates for the selected game using its App ID, even if the game is not actually running. (This is Rich Presence only so Quests will not detect it)

(This may not work for some time, as Discord updates their RPC and SDK. The syntax I used in the Rust code may not be updated, as it is not one of the main focus of this app.)

Though this is functional as it uses Discord Rich Presence, what happens is it uses the App ID of some App on Discord and connects it to the RPC.  
See: [Discord Developer Docs – Creating an App](https://discord.com/developers/docs/quick-start/getting-started#step-1-creating-an-app)

For example, the App ID for Overwatch is `356875221078245376`, and we use it with something like [discordjs/rpc](https://github.com/discordjs/RPC).


```js
// Set this to your Client ID.
const clientId = '356875221078245376'; // This is Overwatch's App ID on Discord, not one I created.
DiscordRPC.register(clientId);

const rpc = new DiscordRPC.Client({ transport: 'ipc' });
const startTimestamp = new Date();

// You will see "Verified Overwatch" on Discord Activity with custom details.
rpc.setActivity({
  details: `Bleet bleet`,
  state: 'in bleet bleet party',
  startTimestamp,
});
```

This may not be the intended use of Discord's RPC and may violate their Terms of Service.  
I am not entirely sure if you can use others' App IDs other than the ones you own in the Discord Developer Dashboard for the application you are developing.
Use this feature at your own risk.


### Disclaimer

This tool is intended for educational purposes and personal use. Please respect Discord's terms of service, partners, game publishers and advertisers rights when using this application.

The creators and maintainers of this project are not liable for any damages, account suspensions, or other consequences that may arise from using this software. Use at your own risk.

Discord is a registered trademark of Discord Inc. It is referenced on this open-source project for descriptive and definition purposes only and does not imply any affiliation, sponsorship, or endorsement by Discord Inc in any way.

---

<!--
## Other Alternatives

If you can't install this application for any reason, there is some steps on a gist from [aamiaa](https://github.com/aamiaa/) that allows you to use Discord client's Web Inspector and paste the code provided and complete the quest.

See the guide here: https://gist.github.com/aamiaa/204cd9d42013ded9faf646fae7f89fbb
-->


## License

[MIT License](LICENSE)© Mark Terence Tiglao - 2025

---


[github-badge]: https://img.shields.io/badge/-Github%20Sponsor-fafbfc?logo=GitHub%20Sponsors
[github-sponsors]: https://github.com/sponsors/markterence
[paypal-badge]: https://img.shields.io/badge/-Paypal-002991?logo=Paypal
[paypal]: https://paypal.me/MarkTerenceTiglao

