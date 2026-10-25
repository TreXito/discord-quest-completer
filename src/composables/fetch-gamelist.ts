import { Game } from '@/types/types';
import { tryOnMounted, useAsyncState } from '@vueuse/core';
import { ref, watch } from 'vue';
import { message } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useGlobalState } from './app-state';

/**
 * The detection list is fetched from three places, best first:
 *
 *   1. `cdn.discordapp.com/detectables/games-v1.json` - the file the Discord
 *      client itself downloads (24883 apps at the time of writing, including
 *      every platform's executables). This is the one that matters, because it
 *      is what the client and the quest system match running processes against.
 *   2. the project's GitHub mirror, same data, no Discord rate limit.
 *   3. `/applications/detectable` from the public API, a smaller older mirror.
 *
 * The bundled snapshot in `src/assets/gamelist.json` is only a last resort.
 */
export interface GameListSource {
    key: 'cdn' | 'mirror' | 'discord' | 'bundled';
    label: string;
    load: () => Promise<unknown>;
}

async function invokeJson(command: string): Promise<unknown> {
    const raw = await invoke(command);
    return decodePayload(raw);
}

/**
 * Rust answers with `tauri::ipc::Response`, i.e. raw bytes. Older code treated
 * that as an array and silently fell through to the bundled list, so handle
 * bytes, string and already-parsed JSON here.
 */
export function decodePayload(raw: unknown): unknown {
    if (raw == null) return undefined;
    if (Array.isArray(raw)) return raw;
    if (typeof raw === 'string') {
        try {
            return JSON.parse(raw);
        } catch {
            return undefined;
        }
    }
    if (raw instanceof ArrayBuffer) {
        try {
            return JSON.parse(new TextDecoder('utf-8').decode(new Uint8Array(raw)));
        } catch {
            return undefined;
        }
    }
    if (ArrayBuffer.isView(raw)) {
        try {
            const view = raw as ArrayBufferView;
            return JSON.parse(new TextDecoder('utf-8').decode(view as Uint8Array));
        } catch {
            return undefined;
        }
    }
    return raw;
}

export function isValidGameList(data: any): boolean {
    return (
        Array.isArray(data) &&
        data.length > 0 &&
        data[0] &&
        'id' in data[0] &&
        'name' in data[0] &&
        'executables' in data[0]
    );
}

export function useFetchGameList() {
    const { addLog } = useGlobalState();

    const sources: GameListSource[] = [
        { key: 'cdn', label: 'Discord detection list', load: () => invokeJson('fetch_gamelist_from_cdn') },
        { key: 'mirror', label: 'GitHub mirror', load: () => invokeJson('fetch_gamelist_gh_mirror') },
        { key: 'discord', label: 'Discord API', load: () => invokeJson('fetch_gamelist_from_discord') },
    ];

    // the bundled snapshot is imported synchronously and always available
    const bundled = useAsyncState(
        () => {
            addLog('Fetching bundled game list for fallback...');
            return import('../assets/gamelist.json').then(res => res.default as unknown as Game[]);
        },
        [],
        { immediate: false, resetOnExecute: true }
    );

    const cdn = useAsyncState<unknown>(sources[0].load, undefined, { immediate: false, resetOnExecute: true });
    const mirror = useAsyncState<unknown>(sources[1].load, undefined, { immediate: false, resetOnExecute: true });
    const discord = useAsyncState<unknown>(sources[2].load, undefined, { immediate: false, resetOnExecute: true });

    const fetchError = ref<string | null>(null);
    const gameDB = ref<Game[]>([]);
    const allFetchDone = ref(false);
    const gameListSource = ref<GameListSource['key'] | null>(null);

    async function trySource(
        state: typeof cdn,
        key: GameListSource['key'],
        label: string
    ): Promise<Game[] | null> {
        try {
            await state.execute();
        } catch (error) {
            addLog('debug', `${label} failed: ${String(error)}`);
            return null;
        }
        const data = state.state.value;
        if (isValidGameList(data)) {
            const list = data as Game[];
            addLog('info', `${label}: ${list.length} applications.`);
            gameListSource.value = key;
            return list;
        }
        addLog('debug', `${label} returned unusable data.`);
        return null;
    }

    async function fetchGameList() {
        allFetchDone.value = false;
        fetchError.value = null;
        gameListSource.value = null;

        // bundled is local, start it so the UI has something either way
        const bundledPromise = bundled.execute().catch(() => undefined);

        let list = await trySource(cdn, 'cdn', 'Discord detection list');
        if (!list) list = await trySource(mirror, 'mirror', 'GitHub mirror');
        if (!list) list = await trySource(discord, 'discord', 'Discord API');

        if (!list) {
            await bundledPromise;
            if (isValidGameList(bundled.state.value)) {
                list = bundled.state.value as Game[];
                gameListSource.value = 'bundled';
                addLog('warning', `Using bundled game list: ${list.length} applications (may be stale).`);
            }
        }

        if (!list) {
            fetchError.value = 'No game list could be loaded (network and bundled list both failed).';
            addLog('error', fetchError.value);
            await message(fetchError.value, {
                title: 'Game List Fetch Error',
                kind: 'error',
                buttons: { ok: 'OK' },
            });
            allFetchDone.value = true;
            return;
        }

        gameDB.value = list;
        addLog('info', `Game database ready: ${list.length} applications (${gameListSource.value}).`);

        // let the notifications breathe for a moment before hiding the panel
        setTimeout(() => {
            allFetchDone.value = true;
        }, 1200);
    }

    watch(allFetchDone, value => {
        if (value) {
            addLog('debug', 'game list fetch finished');
        }
    });

    tryOnMounted(async () => {
        await fetchGameList();
    });

    return {
        gameDB,
        gameListSource,
        fetchError,
        fetchGameList,
        allFetchDone,
        // primary source (cdn, with the mirror and the API as fallbacks)
        isReadyPrimary: cdn.isReady,
        isLoadingPrimary: cdn.isLoading,
        // kept for the status panel
        isReadyGH: mirror.isReady,
        isLoadingGH: mirror.isLoading,
        isReadyDiscord: discord.isReady,
        isLoadingDiscord: discord.isLoading,
        isReadyBundled: bundled.isReady,
        isLoadingBundled: bundled.isLoading,
    };
}
