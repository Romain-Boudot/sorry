<template>
  <div class="settings-body">
    <div v-if="loading" class="stats-loading">
      <Loader2 :size="20" class="spinner" /> Chargement...
    </div>
    <template v-else-if="stats">
      <!-- Header -->
      <div class="stats-header">
        <span class="stats-version">Sorry v{{ stats.version }}</span>
        <span class="stats-uptime">Uptime: {{ formatUptime(stats.uptime_secs) }}</span>
        <button class="stats-refresh" @click="load" title="Rafraichir">
          <RefreshCw :size="14" />
        </button>
      </div>

      <!-- Grid -->
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-icon"><Users :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ stats.users_total }}</span>
            <span class="stat-label">Utilisateurs</span>
          </div>
          <div class="stat-details">
            <span>{{ stats.users_online }} en ligne</span>
            <span v-if="stats.users_guests">{{ stats.users_guests }} invites</span>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon"><MessageSquare :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ formatNumber(stats.messages_total) }}</span>
            <span class="stat-label">Messages</span>
          </div>
          <div class="stat-details">
            <span>{{ stats.messages_today }} aujourd'hui</span>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon"><Hash :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ stats.channels_text + stats.channels_voice }}</span>
            <span class="stat-label">Channels</span>
          </div>
          <div class="stat-details">
            <span>{{ stats.channels_text }} texte</span>
            <span>{{ stats.channels_voice }} vocal</span>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon"><FileIcon :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ stats.files_total }}</span>
            <span class="stat-label">Fichiers</span>
          </div>
          <div class="stat-details">
            <span>{{ formatSize(stats.files_size_bytes) }}</span>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon"><Shield :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ stats.bans_active }}</span>
            <span class="stat-label">Bans actifs</span>
          </div>
          <div class="stat-details">
            <span>{{ stats.invites_active }} invitation{{ stats.invites_active > 1 ? 's' : '' }}</span>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon"><HardDrive :size="18" /></div>
          <div class="stat-body">
            <span class="stat-value">{{ formatSize(stats.db_size_bytes) }}</span>
            <span class="stat-label">Base de donnees</span>
          </div>
          <div class="stat-details">
            <template v-if="stats.disk_total_bytes > 0">
              <span>{{ formatSize(stats.disk_free_bytes) }} libre</span>
              <div class="disk-bar">
                <div class="disk-used" :style="{ width: diskUsedPercent + '%' }" :class="{ warning: diskUsedPercent > 80, danger: diskUsedPercent > 95 }"></div>
              </div>
            </template>
            <span v-else>Disque: N/A</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { Loader2, RefreshCw, Users, MessageSquare, Hash, FileIcon, Shield, HardDrive } from "lucide-vue-next";
import { activeServer } from "../../store";
import { api, type ServerStats } from "../../api";

const stats = ref<ServerStats | null>(null);
const loading = ref(false);

const diskUsedPercent = computed(() => {
  if (!stats.value || !stats.value.disk_total_bytes) return 0;
  return Math.round(((stats.value.disk_total_bytes - stats.value.disk_free_bytes) / stats.value.disk_total_bytes) * 100);
});

async function load() {
  const s = activeServer();
  if (!s) return;
  loading.value = true;
  try {
    stats.value = await api.serverStats(s.url, s.token);
  } catch {
    stats.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(load);

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}j ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} Go`;
}

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}
</script>

<style scoped>
.settings-body {
  padding: 0 24px 24px;
  overflow-y: auto;
  flex: 1;
}

.stats-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px;
  color: var(--text-faint);
  font-size: 0.875rem;
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.stats-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.stats-version {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--header-primary);
  background: var(--bg-tertiary);
  padding: 3px 10px;
  border-radius: 10px;
}

.stats-uptime {
  font-size: 0.75rem;
  color: var(--text-faint);
}

.stats-refresh {
  margin-left: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  margin-right: 0;
  padding: 0;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}

.stats-refresh:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
}

/* ── Grid ── */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.stat-card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.stat-icon {
  color: var(--accent);
}

.stat-body {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.stat-value {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--header-primary);
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.stat-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.6875rem;
  color: var(--text-faint);
}

/* ── Disk bar ── */
.disk-bar {
  width: 100%;
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 2px;
}

.disk-used {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.3s;
}

.disk-used.warning {
  background: var(--warning, #f0b232);
}

.disk-used.danger {
  background: var(--danger);
}
</style>
