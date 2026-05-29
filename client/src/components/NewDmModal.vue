<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal-card">
      <div class="modal-header">
        <h3>Nouveau message prive</h3>
        <button class="modal-close" @click="emit('close')" title="Fermer"><X :size="16" /></button>
      </div>

      <div class="search-wrap">
        <Search :size="14" class="search-icon" />
        <input
          ref="inputEl"
          v-model="query"
          class="search-input"
          type="text"
          placeholder="Rechercher un utilisateur..."
        />
      </div>

      <div class="user-list">
        <div v-if="!filtered.length" class="empty">Aucun utilisateur trouve.</div>
        <button
          v-for="u in filtered"
          :key="u.id"
          class="user-row"
          :disabled="!u.public_key"
          :title="u.public_key ? '' : noKeyLabel"
          @click="onPick(u.id)"
        >
          <div class="user-avatar">
            <img v-if="resolveAvatarUrl(u.id)" :src="resolveAvatarUrl(u.id)!" />
            <span v-else>{{ u.display_name[0]?.toUpperCase() }}</span>
          </div>
          <div class="user-meta">
            <div class="user-name">
              {{ u.display_name }}
              <span v-if="u.guest" class="guest-tag">Guest</span>
            </div>
            <div class="user-sub">
              <ShieldCheck v-if="u.public_key" :size="11" class="ok-icon" />
              <KeyRound v-else :size="11" class="warn-icon" />
              <span v-if="u.public_key" class="fingerprint">{{ u.key_fingerprint }}</span>
              <span v-else class="no-key">Pas encore de cle de chiffrement</span>
            </div>
          </div>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { X, Search, ShieldCheck, KeyRound } from "lucide-vue-next";
import { activeState, openDmWith, resolveAvatarUrl } from "../store";

const emit = defineEmits<{ close: [] }>();

const state = computed(() => activeState());
const query = ref("");
const inputEl = ref<HTMLInputElement>();
const noKeyLabel = "Cet utilisateur n'a pas encore de cle de chiffrement";

const filtered = computed(() => {
  const me = state.value?.user?.id;
  const q = query.value.trim().toLowerCase();
  const users = Array.from(state.value?.users.values() ?? []).filter((u) => u.id !== me);
  const matched = q
    ? users.filter((u) => u.display_name.toLowerCase().includes(q) || (u.username ?? "").toLowerCase().includes(q))
    : users;
  // Users with keys first, then alphabetical.
  return matched.sort((a, b) => {
    const aHas = a.public_key ? 0 : 1;
    const bHas = b.public_key ? 0 : 1;
    if (aHas !== bHas) return aHas - bHas;
    return a.display_name.localeCompare(b.display_name);
  });
});

async function onPick(userId: number) {
  await openDmWith(userId);
  emit("close");
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

onMounted(() => {
  document.addEventListener("keydown", onKey);
  inputEl.value?.focus();
});
onUnmounted(() => document.removeEventListener("keydown", onKey));
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-card {
  width: 440px;
  max-width: 92vw;
  max-height: 72vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border-radius: 10px;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 16px 12px;
  border-bottom: 1px solid var(--border);
}
.modal-header h3 {
  font-size: 1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin: 0;
}
.modal-close {
  width: 26px;
  height: 26px;
  padding: 0;
  margin: 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.modal-close:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }

.search-wrap {
  position: relative;
  padding: 12px 16px 8px;
}
.search-icon {
  position: absolute;
  left: 26px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-faint);
  pointer-events: none;
}
.search-input {
  width: 100%;
  padding: 8px 10px 8px 30px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}
.search-input:focus { border-color: var(--accent); }

.user-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 12px;
}
.empty {
  padding: 20px 16px;
  text-align: center;
  color: var(--text-faint);
  font-size: 0.8125rem;
}

.user-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  text-align: left;
  color: var(--text-normal);
  margin: 0;
}
.user-row:hover:not(:disabled) {
  background: var(--bg-modifier-hover);
}
.user-row:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--text-bright);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.8125rem;
  overflow: hidden;
  flex-shrink: 0;
}
.user-avatar img { width: 100%; height: 100%; object-fit: cover; }

.user-meta { flex: 1; min-width: 0; }
.user-name {
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--header-primary);
  display: flex;
  align-items: center;
  gap: 6px;
}
.user-sub {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.6875rem;
  color: var(--text-faint);
  margin-top: 2px;
}
.fingerprint { font-family: monospace; color: var(--green); }
.no-key { font-style: italic; }
.ok-icon { color: var(--green); flex-shrink: 0; }
.warn-icon { color: var(--text-faint); flex-shrink: 0; }
</style>
