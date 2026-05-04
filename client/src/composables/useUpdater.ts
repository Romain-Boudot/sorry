/**
 * Tauri auto-update bridge.
 *
 * - `autoCheckOnStartup()` : silent check on app boot. Populates `updateState`
 *   so the UI can render a banner when something's available.
 * - `checkForUpdates(silent?)` : manual trigger (Settings → About button).
 * - `installUpdate()` : downloads, applies, relaunches.
 *
 * All functions are safe to call in the web build — they no-op when Tauri APIs
 * are absent, so the same call sites work in both shells.
 */
import { ref } from "vue";
import { showToast } from "./useToast";

export interface PendingUpdate {
  version: string;
  notes?: string;
}

export const pendingUpdate = ref<PendingUpdate | null>(null);
export const installing = ref(false);
export const installProgress = ref(0);

const isTauri = "__TAURI_INTERNALS__" in window;

// Plugin returns an Update handle we need to keep around for downloadAndInstall.
// Stored outside the reactive ref so Vue doesn't try to deeply observe it.
let updateHandle: unknown = null;

export async function checkForUpdates(silent = false): Promise<void> {
  if (!isTauri) {
    if (!silent) showToast("Mises a jour disponibles uniquement sur l'app desktop.", "info");
    return;
  }
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (update) {
      updateHandle = update;
      pendingUpdate.value = { version: update.version, notes: update.body ?? undefined };
      if (!silent) showToast(`Mise a jour disponible : v${update.version}`, "info", 4000);
    } else {
      updateHandle = null;
      pendingUpdate.value = null;
      if (!silent) showToast("Tu es deja a jour.", "success");
    }
  } catch (e) {
    if (!silent) showToast(`Echec de la verification : ${e}`, "error", 4000);
  }
}

export async function installUpdate(): Promise<void> {
  if (!updateHandle || installing.value) return;
  installing.value = true;
  installProgress.value = 0;
  try {
    type ProgressEvent =
      | { event: "Started"; data: { contentLength?: number } }
      | { event: "Progress"; data: { chunkLength: number } }
      | { event: "Finished" };
    let downloaded = 0;
    let total = 0;
    // The `Update` handle exposes downloadAndInstall(onProgress).
    await (updateHandle as { downloadAndInstall: (cb: (e: ProgressEvent) => void) => Promise<void> })
      .downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        else if (e.event === "Progress") {
          downloaded += e.data.chunkLength;
          if (total > 0) installProgress.value = downloaded / total;
        } else if (e.event === "Finished") {
          installProgress.value = 1;
        }
      });
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (e) {
    showToast(`Echec de l'install : ${e}`, "error", 5000);
    installing.value = false;
  }
}

export function autoCheckOnStartup(): void {
  if (!isTauri) return;
  // Small delay so the initial connect/snapshot work isn't competing for
  // network/CPU with the updater check.
  setTimeout(() => { void checkForUpdates(true); }, 3000);
}
