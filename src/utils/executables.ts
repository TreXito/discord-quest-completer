import type { Game, GameExecutable } from '@/types/types';

/**
 * Characters that cannot be part of a fake game file name.
 *
 * On Linux several of these are legal, but the same list drives the Windows
 * build and Discord registers `>hl2.exe` style names which are database hints,
 * not real files (the plain name is always registered too). Refusing them keeps
 * one code path for every platform and never produces a path we cannot recreate
 * on Windows.
 */
const ILLEGAL_NAME_CHARS = ['>', '<', ':', '"', '|', '?', '*', '{', '}'];

export function isLaunchableName(name?: string | null): boolean {
    if (!name) return false;
    if (name.trim() === '') return false;
    if (name.includes('\0')) return false;
    if (name.startsWith('/') || name.startsWith('~')) return false;
    if (name.split(/[/\\]/).some(segment => segment === '..')) return false;
    return !ILLEGAL_NAME_CHARS.some(char => name.includes(char));
}

/** Discord on Linux matches *names*, not platforms, so the order is a preference. */
export function osRank(os?: string | null): number {
    switch (os) {
        case 'linux':
            return 0;
        case 'win32':
            return 1;
        case 'darwin':
            return 2;
        default:
            return 3;
    }
}

export function launchableExecutables(game: Game | null | undefined): GameExecutable[] {
    if (!game?.executables) return [];
    return game.executables
        .filter(executable => isLaunchableName(executable.name))
        .slice()
        .sort((a, b) => {
            const byOs = osRank(a.os) - osRank(b.os);
            if (byOs !== 0) return byOs;
            // entries with required arguments are harder to fake correctly
            const byArgs = Number(!!a.arguments) - Number(!!b.arguments);
            if (byArgs !== 0) return byArgs;
            return a.name.localeCompare(b.name);
        });
}

/**
 * Best guess for "just play this game": native name first, then the windows
 * name, which is what the vast majority of Discord's quest games register.
 */
export function preferredExecutable(game: Game | null | undefined): GameExecutable | null {
    const list = launchableExecutables(game);
    return list.length > 0 ? list[0] : null;
}

export function isCrossPlatformName(executable: GameExecutable, platform: string): boolean {
    return !!executable.os && executable.os !== platform;
}
