/**
 * Per-user stream quality preferences — persisted in localStorage.
 *
 * Two kinds of sources (camera / screen_share) each get their own settings
 * because the sweet spot differs: cameras want low bitrate motion, screens
 * want higher resolution with `detail` hint (or `motion` for gaming).
 */
import { reactive, watch } from "vue";

export type Resolution = "480p" | "720p" | "1080p" | "1440p" | "4k";
export type Fps = 15 | 30 | 60;
export type ContentHint = "motion" | "detail" | "text";
export type PresetId = "eco" | "standard" | "quality";

export interface StreamPreset {
  resolution: Resolution;
  fps: Fps;
  /** Max publish bitrate in kbps. */
  bitrateKbps: number;
  contentHint: ContentHint;
  /** Publish multiple simulcast layers so viewers can adapt to their network.
   *  Disable to push only the high layer — saves upload bandwidth + CPU but
   *  viewers can't degrade gracefully on poor connections. */
  simulcast: boolean;
}

export interface SourceSettings {
  /** Currently selected preset, or "custom" when user has tweaked individual fields. */
  presetId: PresetId | "custom";
  preset: StreamPreset;
}

export interface AllSettings {
  camera: SourceSettings;
  screen: SourceSettings;
}

// ── Preset tables ──

export const CAMERA_PRESETS: Record<PresetId, StreamPreset> = {
  eco:      { resolution: "480p",  fps: 15, bitrateKbps: 500,  contentHint: "motion", simulcast: true },
  standard: { resolution: "720p",  fps: 30, bitrateKbps: 1500, contentHint: "motion", simulcast: true },
  quality:  { resolution: "1080p", fps: 30, bitrateKbps: 3000, contentHint: "motion", simulcast: true },
};

export const SCREEN_PRESETS: Record<PresetId, StreamPreset> = {
  eco:      { resolution: "720p",  fps: 15, bitrateKbps: 1000, contentHint: "detail", simulcast: true },
  standard: { resolution: "1080p", fps: 30, bitrateKbps: 3000, contentHint: "detail", simulcast: true },
  quality:  { resolution: "1080p", fps: 60, bitrateKbps: 6000, contentHint: "motion", simulcast: true },
};

export function presetsFor(kind: "camera" | "screen") {
  return kind === "camera" ? CAMERA_PRESETS : SCREEN_PRESETS;
}

function defaultSettings(): AllSettings {
  return {
    camera: { presetId: "standard", preset: { ...CAMERA_PRESETS.standard } },
    screen: { presetId: "standard", preset: { ...SCREEN_PRESETS.standard } },
  };
}

// ── Storage ──

const STORAGE_KEY = "stream-settings";

function load(): AllSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultSettings();
    const parsed = JSON.parse(raw) as Partial<AllSettings>;
    const fallback = defaultSettings();
    return {
      camera: parsed.camera ?? fallback.camera,
      screen: parsed.screen ?? fallback.screen,
    };
  } catch {
    return defaultSettings();
  }
}

export const streamSettings = reactive<AllSettings>(load());

watch(streamSettings, () => {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(streamSettings)); } catch {}
}, { deep: true });

// ── Helpers ──

export const RESOLUTION_DIMS: Record<Resolution, { width: number; height: number }> = {
  "480p":  { width: 854,  height: 480  },
  "720p":  { width: 1280, height: 720  },
  "1080p": { width: 1920, height: 1080 },
  "1440p": { width: 2560, height: 1440 },
  "4k":    { width: 3840, height: 2160 },
};

/** Available resolutions for a given source.
 *  - Camera: capped at 1080p (very few webcams capture higher and we can't
 *    detect their max without opening the device).
 *  - Screen: all options up to 4K. We can't know which monitor the user will
 *    pick in the browser's source picker (it might be a different display
 *    than the one this app window is on), so gating by `window.screen` would
 *    hide valid choices. If the chosen source is smaller than the requested
 *    resolution, the browser just captures at the source's native size. */
export function availableResolutions(kind: "camera" | "screen"): Resolution[] {
  if (kind === "camera") return ["480p", "720p", "1080p"];
  return ["480p", "720p", "1080p", "1440p", "4k"];
}

/** Recommended bitrate (kbps) for a given (resolution, fps, hint) tuple.
 *  Numbers chosen to match common streaming practice (Discord/Twitch ranges):
 *  motion content gets the full bitrate, detail/text reduce since they
 *  compress better. Values are conservative — viewers rarely complain about
 *  too much bitrate, but they do complain about choppy video. */
const BITRATE_TABLE: Record<Resolution, Record<Fps, number>> = {
  "480p":  { 15: 500,  30: 800,  60: 1200 },
  "720p":  { 15: 1000, 30: 1500, 60: 2500 },
  "1080p": { 15: 2000, 30: 3000, 60: 6000 },
  "1440p": { 15: 3500, 30: 5000, 60: 10000 },
  "4k":    { 15: 8000, 30: 14000, 60: 25000 },
};

const HINT_MULTIPLIER: Record<ContentHint, number> = {
  motion: 1.0,
  detail: 0.7,
  text:   0.5,
};

export function recommendedBitrate(p: { resolution: Resolution; fps: Fps; contentHint: ContentHint }): number {
  const base = BITRATE_TABLE[p.resolution][p.fps] ?? 1500;
  return Math.round(base * HINT_MULTIPLIER[p.contentHint]);
}

/** Check if two presets differ on fields that require re-publishing the track.
 *  Resolution and FPS need a new capture; bitrate and hint can be changed live. */
export function needsRestart(a: StreamPreset, b: StreamPreset): boolean {
  return a.resolution !== b.resolution || a.fps !== b.fps;
}

/** Detect which preset (if any) the current values exactly match. */
export function matchPreset(kind: "camera" | "screen", p: StreamPreset): PresetId | "custom" {
  const table = presetsFor(kind);
  for (const id of ["eco", "standard", "quality"] as PresetId[]) {
    const ref = table[id];
    if (
      ref.resolution === p.resolution &&
      ref.fps === p.fps &&
      ref.bitrateKbps === p.bitrateKbps &&
      ref.contentHint === p.contentHint &&
      ref.simulcast === p.simulcast
    ) return id;
  }
  return "custom";
}

export function applyPreset(kind: "camera" | "screen", id: PresetId) {
  const preset = { ...presetsFor(kind)[id] };
  if (kind === "camera") {
    streamSettings.camera = { presetId: id, preset };
  } else {
    streamSettings.screen = { presetId: id, preset };
  }
}

export function updateField<K extends keyof StreamPreset>(
  kind: "camera" | "screen",
  field: K,
  value: StreamPreset[K],
) {
  const section = kind === "camera" ? streamSettings.camera : streamSettings.screen;
  section.preset = { ...section.preset, [field]: value };
  section.presetId = matchPreset(kind, section.preset);
}
