#!/bin/sh
# Discord Quest Completer - Linux installer.
#
#   curl -fsSL https://raw.githubusercontent.com/TreXito/discord-quest-completer-linux/main/install.sh | sh
#   ./install.sh --local                     # install from this checkout, no download
#   ./install.sh --with-gui                  # also install the desktop app + menu entry
#   ./install.sh --uninstall                 # remove the CLI (+ GUI and menu entry)
#   ./install.sh --uninstall --purge         # ... and delete downloaded game list/state
#
# Installs "dqc" (a single dependency-free Python file) plus the ~11KB runner, so
# no Rust, no Tauri and no WebKit are needed for the CLI path. The GUI is
# optional and only installed with --with-gui.
set -eu

REPO="${DQC_REPO:-TreXito/discord-quest-completer-linux}"
RAW="https://raw.githubusercontent.com/$REPO/main"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/discord-quest-completer"
APPS_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/128x128/apps"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/discord-quest-completer"

MODE=install
LOCAL=0
WITH_GUI=0
PURGE=0
RUN_UPDATE=1

say()  { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
    awk 'NR>1 && /^#/ { sub(/^# ?/, ""); print; next } NR>1 { exit }' "$0" 2>/dev/null || true
    exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --local)     LOCAL=1 ;;
        --with-gui|--gui) WITH_GUI=1 ;;
        --uninstall) MODE=uninstall ;;
        --purge)     PURGE=1 ;;
        --no-update) RUN_UPDATE=0 ;;
        --prefix)    shift; [ $# -gt 0 ] || die "--prefix needs a value"; PREFIX=$1
                     BIN_DIR="$PREFIX/bin"; SHARE_DIR="$PREFIX/share/discord-quest-completer"
                     APPS_DIR="$PREFIX/share/applications"
                     ICON_DIR="$PREFIX/share/icons/hicolor/128x128/apps" ;;
        --prefix=*)  PREFIX=${1#*=}
                     BIN_DIR="$PREFIX/bin"; SHARE_DIR="$PREFIX/share/discord-quest-completer"
                     APPS_DIR="$PREFIX/share/applications"
                     ICON_DIR="$PREFIX/share/icons/hicolor/128x128/apps" ;;
        -h|--help)   usage ;;
        *)           die "unknown argument: $1 (try --help)" ;;
    esac
    shift
done

fetch() { # fetch <url> <dest>
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        die "need curl or wget to download release files"
    fi
}

# ---------------------------------------------------------------- uninstall ---
if [ "$MODE" = uninstall ]; then
    for f in "$BIN_DIR/dqc" "$BIN_DIR/discord-quest-completer" \
             "$APPS_DIR/discord-quest-completer.desktop" \
             "$ICON_DIR/discord-quest-completer.png"; do
        [ -e "$f" ] && rm -f "$f" && say "removed $f"
    done
    [ -d "$SHARE_DIR" ] && rm -rf "$SHARE_DIR" && say "removed $SHARE_DIR"
    if [ "$PURGE" = 1 ] && [ -d "$DATA_DIR" ]; then
        rm -rf "$DATA_DIR" && say "removed data $DATA_DIR (game list, fake games)"
    elif [ -d "$DATA_DIR" ]; then
        say "kept $DATA_DIR (use --purge to delete it)"
    fi
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" 2>/dev/null || true
    say "uninstalled."
    exit 0
fi

# ------------------------------------------------------------------- checks ---
arch=$(uname -m)
[ "$arch" = "x86_64" ] || die "prebuilt $arch builds do not exist yet (only x86_64); build from source: cd src-linux && ./build.sh --copy && pnpm build:linux"

SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || true)
if [ "$LOCAL" = 0 ] && [ -n "${SELF_DIR:-}" ] && [ -f "$SELF_DIR/cli/dqc" ]; then
    LOCAL=1
    say "detected a checkout at $SELF_DIR, installing from it (--local)"
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

# ------------------------------------------------------------------ payload ---
if [ "$LOCAL" = 1 ]; then
    SRC="${SELF_DIR:-.}"
    [ -f "$SRC/cli/dqc" ] || die "--local needs cli/dqc (run this from the repo root)"

    CLI_SRC="$SRC/cli/dqc"
    RUNNER_SRC=""
    for cand in "$SRC/src-linux/src-linux-bin" "$SRC/src-linux/target/release/src-linux" \
                "$SRC/src-tauri/resources/src-linux"; do
        [ -f "$cand" ] && { RUNNER_SRC=$cand; break; }
    done
    if [ -z "$RUNNER_SRC" ]; then
        say "no runner built yet, building it (cc, ~11KB, links only libc)..."
        if [ -x "$SRC/src-linux/build.sh" ]; then
            ( cd "$SRC/src-linux" && FORCE_CC=1 ./build.sh ) || die "runner build failed"
            for cand in "$SRC/src-linux/src-linux-bin" "$SRC/src-linux/target/release/src-linux"; do
                [ -f "$cand" ] && { RUNNER_SRC=$cand; break; }
            done
        fi
    fi
    [ -n "$RUNNER_SRC" ] || die "could not find or build the runner"

    GUI_SRC=""
    for cand in "$SRC/src-tauri/target/release/discord-quest-completer"; do
        [ -f "$cand" ] && { GUI_SRC=$cand; break; }
    done
    [ "$WITH_GUI" = 1 ] && [ -z "$GUI_SRC" ] && die "--with-gui needs a built app: pnpm build:linux"
    ICON_SRC="$SRC/src-tauri/icons/128x128.png"
else
    VERSION="${DQC_VERSION:-}"
    if [ -z "$VERSION" ]; then
        say "resolving latest release..."
        fetch "https://api.github.com/repos/$REPO/releases/latest" "$TMP/rel.json" \
            || die "could not reach the GitHub API; set DQC_VERSION=<tag> or use --local"
        VERSION=$(sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$TMP/rel.json" | head -1)
        [ -n "$VERSION" ] || die "no release found; use --local to install from a checkout"
    fi
    BASE="https://github.com/$REPO/releases/download/$VERSION"
    say "installing $VERSION"

    fetch "$BASE/discord-quest-completer-cli-linux-x86_64.tar.gz" "$TMP/cli.tar.gz" \
        || die "release $VERSION has no CLI tarball (needs v2 release assets)"
    mkdir -p "$TMP/cli"
    tar -xzf "$TMP/cli.tar.gz" -C "$TMP/cli"
    CLI_SRC=$(find "$TMP/cli" -maxdepth 2 -name dqc -type f | head -1)
    RUNNER_SRC=$(find "$TMP/cli" -maxdepth 2 -name src-linux -type f | head -1)
    [ -n "$CLI_SRC" ] && [ -n "$RUNNER_SRC" ] || die "CLI tarball is missing dqc or src-linux"

    GUI_SRC=""
    if [ "$WITH_GUI" = 1 ]; then
        fetch "$BASE/discord-quest-completer-linux-x86_64.AppImage" "$TMP/app.AppImage" \
            || die "release $VERSION has no AppImage"
        GUI_SRC="$TMP/app.AppImage"
    fi
    fetch "$BASE/icon-128.png" "$TMP/icon.png" 2>/dev/null || true
    ICON_SRC="$TMP/icon.png"
fi

# ------------------------------------------------------------------ install ---
mkdir -p "$BIN_DIR" "$SHARE_DIR" "$DATA_DIR/runner" "$DATA_DIR/games"

install -m 0644 "$CLI_SRC" "$SHARE_DIR/dqc"
install -m 0755 "$RUNNER_SRC" "$DATA_DIR/runner/src-linux"

# Thin wrapper so dqc is on PATH and still resolves its own directory correctly.
cat > "$BIN_DIR/dqc" <<EOF
#!/bin/sh
exec python3 "$SHARE_DIR/dqc" "\$@"
EOF
chmod 0755 "$BIN_DIR/dqc"

say "installed dqc        $BIN_DIR/dqc"
say "installed runner     $DATA_DIR/runner/src-linux ($(wc -c <"$DATA_DIR/runner/src-linux") bytes)"

if [ "$WITH_GUI" = 1 ] && [ -n "$GUI_SRC" ]; then
    mkdir -p "$APPS_DIR" "$ICON_DIR"
    install -m 0755 "$GUI_SRC" "$BIN_DIR/discord-quest-completer"
    [ -f "${ICON_SRC:-}" ] && install -m 0644 "$ICON_SRC" "$ICON_DIR/discord-quest-completer.png"

    cat > "$APPS_DIR/discord-quest-completer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Discord Quest Completer
Comment=Fake any game Discord detects, for quests and rich presence
Exec=$BIN_DIR/discord-quest-completer
Icon=$ICON_DIR/discord-quest-completer.png
Terminal=false
Categories=Utility;Game;
StartupWMClass=discord-quest-completer
EOF
    chmod 0644 "$APPS_DIR/discord-quest-completer.desktop"
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" 2>/dev/null || true
    say "installed GUI        $BIN_DIR/discord-quest-completer"
    say "installed menu entry $APPS_DIR/discord-quest-completer.desktop"
fi

if [ "$RUN_UPDATE" = 1 ]; then
    say ""
    say "fetching Discord's detection list..."
    "$BIN_DIR/dqc" update || warn "could not fetch the game list now; run 'dqc update' later"
fi

say ""
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        warn "$BIN_DIR is not on PATH; add it, e.g.:"
        say "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.profile"
        ;;
esac

say "done. try:"
say "  dqc ls -n 5"
say "  dqc play overwatch --for 15m"
say "  dqc status"
say "  dqc stop --all"
say ""
say "note: Discord's own client does the detecting. It must be running and logged in,"
say "      and it must be the official desktop build (Vesktop/Web builds ship no"
say "      detection module, so fake games are never seen)."
