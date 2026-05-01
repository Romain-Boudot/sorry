<template>
  <div class="modal-overlay" @click.self="close" @keydown.esc.window="close">
    <div class="modal">
      <div class="server-header">
        <div class="server-header-icon">
          <img v-if="iconUrl" :src="iconUrl" />
          <span v-else>{{ server?.name?.[0]?.toUpperCase() }}</span>
        </div>
        <div>
          <h2>Session expiree</h2>
          <p class="server-header-desc">{{ server?.name }}</p>
        </div>
      </div>

      <p class="step-desc">
        Ton jeton de session sur ce serveur a expire. Re-saisis ton mot de passe pour relancer la connexion.
      </p>

      <div class="field">
        <label>Identifiant</label>
        <input :value="server?.username" type="text" disabled />
      </div>

      <div class="field">
        <label>Mot de passe</label>
        <input
          v-model="password"
          type="password"
          placeholder="********"
          autofocus
          @keydown.enter="submit"
        />
      </div>

      <template v-if="needsTotp">
        <div class="field">
          <label>Code TOTP</label>
          <input
            v-model="totpCode"
            type="text"
            inputmode="numeric"
            pattern="[0-9]*"
            maxlength="6"
            placeholder="000000"
            @keydown.enter="submit"
          />
        </div>
      </template>

      <p class="error" v-if="error">{{ error }}</p>

      <div class="step-actions">
        <button class="btn-back" @click="close">Annuler</button>
        <button
          @click="submit"
          :disabled="loading || !password.trim() || (needsTotp && totpCode.length < 6)"
        >
          {{ loading ? "Connexion..." : "Se reconnecter" }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { store, persistServers } from "../store";
import { connectToServer } from "../composables/useConnection";
import { api } from "../api";

const password = ref("");
const totpCode = ref("");
const needsTotp = ref(false);
const loading = ref(false);
const error = ref("");

const server = computed(() =>
  store.savedServers.find((s) => s.id === store.reauthServerId),
);

const iconUrl = computed(() => {
  const s = server.value;
  return s?.iconUrl ? `${s.url}${s.iconUrl}` : null;
});

async function submit() {
  const s = server.value;
  if (!s) return;
  error.value = "";
  loading.value = true;
  try {
    const res = await api.login(
      s.url,
      s.username,
      password.value,
      undefined,
      needsTotp.value ? totpCode.value : undefined,
    );

    if (res.totp_required) {
      needsTotp.value = true;
      return;
    }

    if (!res.token) {
      error.value = "Reponse invalide du serveur";
      return;
    }

    s.token = res.token;
    persistServers();

    const serverId = s.id;
    close();
    // On evite l'etat zombie laisse par le precedent connect avorte.
    store.serverStates.delete(serverId);
    await connectToServer(serverId);
    store.activeServerId = serverId;
  } catch (e: any) {
    if (e?.message === "totp_required") {
      needsTotp.value = true;
    } else if (e?.message === "401") {
      error.value = needsTotp.value ? "Code TOTP incorrect" : "Mot de passe incorrect";
      totpCode.value = "";
    } else if (e?.message === "403") {
      error.value = "Acces refuse";
    } else if (e?.message === "429") {
      error.value = "Trop de tentatives, reessaie dans une minute";
    } else {
      error.value = "Erreur de connexion";
    }
  } finally {
    loading.value = false;
  }
}

function close() {
  store.reauthServerId = null;
  password.value = "";
  totpCode.value = "";
  needsTotp.value = false;
  error.value = "";
}
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

.modal {
  background: var(--bg-primary);
  padding: 32px;
  border-radius: 8px;
  width: 460px;
  max-height: 90vh;
  overflow-y: auto;
}

.modal h2 {
  margin-bottom: 4px;
  font-weight: 700;
  font-size: 1.25rem;
  color: var(--header-primary);
}

.server-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.server-header-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: var(--accent);
  color: var(--text-bright);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1.25rem;
  overflow: hidden;
  flex-shrink: 0;
}

.server-header-icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.server-header-desc {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin: 2px 0 0;
}

.step-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin-bottom: 16px;
}

.field {
  margin-bottom: 12px;
}

.field label {
  display: block;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.field input {
  width: 100%;
  padding: 10px 12px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-normal);
  font-size: 0.875rem;
}

.field input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.field input:focus {
  outline: none;
  border-color: var(--accent);
}

.error {
  color: var(--danger);
  font-size: 0.8125rem;
  margin: 8px 0 0;
}

.step-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}

.btn-back {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border);
}

.btn-back:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}
</style>
