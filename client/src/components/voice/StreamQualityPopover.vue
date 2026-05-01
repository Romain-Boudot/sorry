<template>
  <Teleport to="body">
    <div class="sqp-overlay" @click.self="$emit('close')">
      <div class="sqp-modal" @click.stop>
        <div class="sqp-header">
          <div class="sqp-header-title">
            <component :is="kind === 'camera' ? Video : Monitor" :size="18" />
            <span>{{ kind === "camera" ? "Qualite webcam" : "Qualite ecran" }}</span>
          </div>
          <button class="sqp-close" @click="$emit('close')" title="Fermer">
            <X :size="16" />
          </button>
        </div>

        <div class="sqp-body">
          <!-- Presets -->
          <div class="sqp-section">
            <div class="sqp-section-title">Mode rapide</div>
            <div class="sqp-presets">
              <button
                v-for="p in presetList"
                :key="p.id"
                class="sqp-preset"
                :class="{ active: settings.presetId === p.id }"
                @click="onPickPreset(p.id)"
              >
                <component :is="p.icon" :size="18" />
                <span class="sqp-preset-label">{{ p.label }}</span>
                <span class="sqp-preset-sub">{{ presetSummary(p.id) }}</span>
              </button>
            </div>
          </div>

          <!-- Advanced fields -->
          <div class="sqp-section">
            <div class="sqp-section-title">
              Avance
              <span v-if="settings.presetId === 'custom'" class="sqp-custom-badge">personnalise</span>
            </div>

            <div class="sqp-row">
              <label>Resolution</label>
              <select :value="settings.preset.resolution" @change="onField('resolution', ($event.target as HTMLSelectElement).value as Resolution)">
                <option v-for="r in resolutionOptions" :key="r" :value="r">{{ resolutionLabel(r) }}</option>
              </select>
            </div>

            <div class="sqp-row">
              <label>Images / seconde</label>
              <div class="sqp-segmented">
                <button
                  v-for="f in [15, 30, 60] as Fps[]"
                  :key="f"
                  class="sqp-seg"
                  :class="{ active: settings.preset.fps === f }"
                  @click="onField('fps', f)"
                >{{ f }}</button>
              </div>
            </div>

            <div class="sqp-row column">
              <div class="sqp-row-header">
                <label>Bitrate maximum</label>
                <span class="sqp-slider-value">{{ formatBitrate(settings.preset.bitrateKbps) }}</span>
              </div>
              <input
                type="range"
                :min="200"
                :max="bitrateMax"
                :step="100"
                :value="settings.preset.bitrateKbps"
                @input="onField('bitrateKbps', Number(($event.target as HTMLInputElement).value))"
              />
            </div>

            <div v-if="kind === 'screen'" class="sqp-row column">
              <label>Type de contenu</label>
              <div class="sqp-hints">
                <button
                  v-for="h in HINT_OPTIONS"
                  :key="h.value"
                  class="sqp-hint"
                  :class="{ active: settings.preset.contentHint === h.value }"
                  @click="onField('contentHint', h.value)"
                >
                  <span class="sqp-hint-label">{{ h.label }}</span>
                  <span class="sqp-hint-desc">{{ h.desc }}</span>
                </button>
              </div>
            </div>

            <div class="sqp-row sqp-toggle-row">
              <div class="sqp-toggle-text">
                <label>Simulcast</label>
                <div class="sqp-toggle-desc">Plusieurs qualites pour les viewers (adaptatif). Desactive pour economiser l'upload.</div>
              </div>
              <button
                class="sqp-toggle"
                :class="{ on: settings.preset.simulcast }"
                @click="onField('simulcast', !settings.preset.simulcast)"
                :title="settings.preset.simulcast ? 'Desactiver simulcast' : 'Activer simulcast'"
              >
                <span class="sqp-toggle-knob" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="isStreaming" class="sqp-status">
          <Loader2 v-if="applying" :size="13" class="sqp-spin" />
          <span v-if="applying">Application des changements...</span>
          <span v-else>Modifications appliquees en direct</span>
        </div>
        <div v-else class="sqp-status sqp-status-muted">
          Les reglages seront appliques au prochain demarrage du {{ kind === 'camera' ? 'webcam' : 'partage ecran' }}
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from "vue";
import { Video, Monitor, Leaf, Gauge, Sparkles, Loader2, X } from "lucide-vue-next";
import {
  streamSettings,
  applyPreset,
  updateField,
  recommendedBitrate,
  availableResolutions,
  RESOLUTION_DIMS,
  type PresetId,
  type Resolution,
  type Fps,
  type ContentHint,
  type StreamPreset,
  presetsFor,
} from "../../streamSettings";
import { applyCameraSettings, applyScreenSettings, isCameraEnabled, isScreenSharing } from "../../voice";

const props = defineProps<{ kind: "camera" | "screen" }>();
const emit = defineEmits<{ close: [] }>();

const settings = computed(() => props.kind === "camera" ? streamSettings.camera : streamSettings.screen);

const presetList = [
  { id: "eco" as PresetId,      label: "Econome",  icon: Leaf },
  { id: "standard" as PresetId, label: "Standard", icon: Gauge },
  { id: "quality" as PresetId,  label: "Qualite",  icon: Sparkles },
];

function presetSummary(id: PresetId): string {
  const p = presetsFor(props.kind)[id];
  return `${p.resolution} ${p.fps}fps`;
}

const resolutionOptions = computed(() => availableResolutions(props.kind));

const bitrateMax = computed(() => {
  if (props.kind === "camera") return 5000;
  // Screen: scale slider ceiling with chosen resolution.
  switch (settings.value.preset.resolution) {
    case "4k":    return 30000;
    case "1440p": return 15000;
    default:      return 10000;
  }
});

function resolutionLabel(r: Resolution): string {
  const dims = RESOLUTION_DIMS[r];
  return `${r === "4k" ? "4K" : r} (${dims.width}x${dims.height})`;
}

const HINT_OPTIONS: { value: ContentHint; label: string; desc: string }[] = [
  { value: "detail", label: "Code / Docs",   desc: "Texte net" },
  { value: "motion", label: "Gaming / Video", desc: "Mouvement fluide" },
  { value: "text",   label: "Texte pur",      desc: "Optimise statique" },
];

function formatBitrate(kbps: number): string {
  if (kbps >= 1000) return `${(kbps / 1000).toFixed(1)} Mbps`;
  return `${kbps} kbps`;
}

const isStreaming = computed(() => props.kind === "camera" ? isCameraEnabled() : isScreenSharing());

const applying = ref(false);

async function applyChanges() {
  if (!isStreaming.value) return;
  applying.value = true;
  try {
    if (props.kind === "camera") await applyCameraSettings();
    else await applyScreenSettings();
  } finally {
    applying.value = false;
  }
}

function onPickPreset(id: PresetId) {
  // Don't call applyChanges here — the watcher below picks up the change
  // and runs it through the 250ms debounce. Calling it now would double-apply.
  applyPreset(props.kind, id);
}

function onField<K extends keyof StreamPreset>(field: K, value: StreamPreset[K]) {
  const wasPreset = settings.value.presetId !== "custom";
  updateField(props.kind, field, value);
  // When the user is on a preset, tweaking a non-bitrate field implies a
  // different optimal bitrate — refresh it so the value stays coherent. Once
  // they're in custom mode (i.e. they moved the slider), respect their
  // bitrate choice and don't overwrite it on subsequent edits.
  if (field !== "bitrateKbps" && wasPreset) {
    updateField(props.kind, "bitrateKbps", recommendedBitrate(settings.value.preset));
  }
}

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
watch(() => settings.value.preset, () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(applyChanges, 250);
}, { deep: true });

// Esc to close
function onKey(e: KeyboardEvent) { if (e.key === "Escape") emit("close"); }
onMounted(() => document.addEventListener("keydown", onKey));
onUnmounted(() => document.removeEventListener("keydown", onKey));
</script>

<style scoped>
.sqp-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.sqp-modal {
  background: var(--bg-primary);
  border-radius: 10px;
  width: 460px;
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 64px);
  display: flex;
  flex-direction: column;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

/* ── Header ── */
.sqp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  background: var(--bg-secondary);
  flex-shrink: 0;
}
.sqp-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 0.9375rem;
  color: var(--header-primary);
}
.sqp-close {
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
  border: none;
  border-radius: 6px;
  cursor: pointer;
}
.sqp-close:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }

/* ── Body ── */
.sqp-body {
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.sqp-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sqp-section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
}

.sqp-custom-badge {
  font-size: 0.625rem;
  padding: 1px 6px;
  background: var(--accent);
  color: var(--text-bright);
  border-radius: 8px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

/* ── Presets ── */
.sqp-presets {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.sqp-preset {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 14px 8px;
  margin: 0;
  background: var(--bg-secondary);
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.1s, border-color 0.1s, color 0.1s;
}
.sqp-preset:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }
.sqp-preset.active {
  background: var(--bg-tertiary);
  border-color: var(--accent);
  color: var(--text-bright);
}
.sqp-preset-label {
  font-size: 0.8125rem;
  font-weight: 600;
}
.sqp-preset-sub {
  font-size: 0.6875rem;
  opacity: 0.65;
  font-variant-numeric: tabular-nums;
}

/* ── Rows ── */
.sqp-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.sqp-row.column {
  flex-direction: column;
  align-items: stretch;
  gap: 6px;
}
.sqp-row-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.sqp-row label {
  font-size: 0.8125rem;
  color: var(--text-normal);
  flex-shrink: 0;
}

.sqp-row select {
  background: var(--bg-secondary);
  color: var(--text-normal);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 0.8125rem;
  padding: 6px 10px;
  outline: none;
  cursor: pointer;
  min-width: 160px;
}

.sqp-row input[type="range"] {
  width: 100%;
  accent-color: var(--accent);
}

.sqp-slider-value {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

/* ── Segmented (FPS) ── */
.sqp-segmented {
  display: inline-flex;
  background: var(--bg-secondary);
  border-radius: 6px;
  padding: 2px;
  gap: 2px;
}
.sqp-seg {
  width: auto;
  padding: 4px 14px;
  margin: 0;
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 0.8125rem;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
}
.sqp-seg:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }
.sqp-seg.active {
  background: var(--bg-tertiary);
  color: var(--text-bright);
}

/* ── Hint cards ── */
.sqp-hints {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}
.sqp-hint {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: 8px 10px;
  margin: 0;
  background: var(--bg-secondary);
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-muted);
  cursor: pointer;
  text-align: left;
}
.sqp-hint:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }
.sqp-hint.active {
  background: var(--bg-tertiary);
  border-color: var(--accent);
  color: var(--text-bright);
}
.sqp-hint-label {
  font-size: 0.75rem;
  font-weight: 600;
}
.sqp-hint-desc {
  font-size: 0.625rem;
  opacity: 0.65;
}

/* ── Toggle (simulcast etc) ── */
.sqp-toggle-row {
  align-items: center;
  gap: 12px;
}
.sqp-toggle-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.sqp-toggle-desc {
  font-size: 0.6875rem;
  color: var(--text-faint);
  line-height: 1.3;
}

.sqp-toggle {
  width: 40px;
  height: 22px;
  padding: 2px;
  margin: 0;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 11px;
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: background 0.15s, border-color 0.15s;
}
.sqp-toggle.on {
  background: var(--accent);
  border-color: var(--accent);
}
.sqp-toggle-knob {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-bright);
  transition: transform 0.15s;
}
.sqp-toggle.on .sqp-toggle-knob { transform: translateX(18px); }

/* ── Status footer ── */
.sqp-status {
  padding: 10px 20px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  font-size: 0.75rem;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  flex-shrink: 0;
}
.sqp-status-muted { color: var(--text-faint); }
.sqp-spin { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
