<template>
  <div class="modal-overlay" @click.self="close" @keydown.esc.window="close">
    <div class="modal">
      <!-- Step 1: Server address -->
      <template v-if="step === 1">
        <h2>Ajouter un serveur</h2>
        <p class="step-desc">Entre l'adresse du serveur que tu veux rejoindre.</p>

        <div class="field">
          <label>Adresse</label>
          <input
            v-model="url"
            type="text"
            placeholder="http://localhost:3000"
            required
            autofocus
            @keydown.enter="nextStep"
          />
        </div>

        <p class="error" v-if="error">{{ error }}</p>

        <div class="step-actions">
          <button @click="nextStep" :disabled="loading || !url.trim()">
            {{ loading ? "Connexion..." : "Suivant" }}
          </button>
        </div>
      </template>

      <!-- Step 2: Login -->
      <template v-if="step === 2">
        <h2>{{ serverName }}</h2>
        <p class="step-desc">Ton identifiant est prive et sert uniquement a te connecter. Les autres verront ton display name.</p>

        <div class="field-group">
          <div class="field">
            <label>Identifiant</label>
            <input v-model="username" type="text" placeholder="ton_id" required autofocus />
          </div>
          <div class="field">
            <label>Mot de passe</label>
            <input v-model="password" type="password" placeholder="********" required @keydown.enter="!showServerPassword && nextStep()" />
          </div>
        </div>

        <template v-if="showServerPassword">
          <div class="field-group">
            <div class="field">
              <label>Display name</label>
              <input v-model="displayName" type="text" placeholder="Ton nom visible" />
            </div>
            <div class="field">
              <label>Mot de passe serveur</label>
              <input v-model="serverPassword" type="password" placeholder="Demande a l'admin" @keydown.enter="nextStep" />
            </div>
          </div>
        </template>

        <p class="toggle" @click="showServerPassword = !showServerPassword">
          {{ showServerPassword ? "J'ai deja un compte" : "Premiere connexion ?" }}
        </p>

        <p class="error" v-if="error">{{ error }}</p>

        <div class="step-actions">
          <button class="btn-back" @click="step = 1">Retour</button>
          <button @click="nextStep" :disabled="loading || !username.trim() || !password.trim()">
            {{ loading ? "Connexion..." : "Se connecter" }}
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { store, addServer } from "../store";
import { api } from "../api";

const step = ref(1);
const loading = ref(false);
const error = ref("");

// Step 1
const url = ref("http://localhost:3000");
const serverName = ref("");

// Step 2
const username = ref("");
const password = ref("");
const displayName = ref(localStorage.getItem("defaultDisplayName") || "");
const serverPassword = ref("");
const showServerPassword = ref(false);

// Returns the URL(s) to try in order. If the user typed an explicit protocol,
// we respect it and try only that. If no protocol, we try https first, then
// http — but only if we're not in a browser served over https (mixed content).
function candidateUrls(raw: string): string[] {
  const trimmed = raw.trim().replace(/\/+$/, "");
  if (/^https?:\/\//i.test(trimmed)) return [trimmed];
  const canTryHttp = window.location.protocol !== "https:"; // tauri: or http: → ok
  return canTryHttp ? [`https://${trimmed}`, `http://${trimmed}`] : [`https://${trimmed}`];
}

async function nextStep() {
  error.value = "";
  loading.value = true;

  try {
    if (step.value === 1) {
      const candidates = candidateUrls(url.value);
      let info: { name: string } | null = null;
      for (const candidate of candidates) {
        try {
          info = await api.serverInfo(candidate);
          url.value = candidate;
          break;
        } catch {
          // try next
        }
      }
      if (!info) throw new Error("unreachable");
      serverName.value = info.name;
      step.value = 2;
    } else if (step.value === 2) {
      await addServer(
        serverName.value,
        url.value,
        username.value,
        password.value,
        showServerPassword.value ? serverPassword.value : undefined,
        showServerPassword.value && displayName.value.trim() ? displayName.value.trim() : undefined
      );
      close();
    }
  } catch (e: any) {
    if (step.value === 1) {
      error.value = "Impossible de joindre ce serveur";
    } else if (e.message === "401") {
      error.value = "Mot de passe incorrect";
    } else if (e.message === "403") {
      error.value = "Mot de passe serveur incorrect";
    } else {
      error.value = "Erreur de connexion";
    }
  } finally {
    loading.value = false;
  }
}

function close() {
  store.showAddServerModal = false;
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
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

.step-desc {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin-bottom: 20px;
}

.field-group {
  display: flex;
  gap: 10px;
  margin-bottom: 14px;
}

.field-group .field {
  flex: 1;
  margin-bottom: 0;
}

.field {
  margin-bottom: 16px;
}

.field label {
  display: block;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.field input {
  width: 100%;
  padding: 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.9375rem;
  font-family: inherit;
  outline: none;
}

.field input::placeholder {
  color: var(--text-faint);
}

.toggle {
  text-align: center;
  color: var(--accent);
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
  margin-bottom: 16px;
}

.toggle:hover { text-decoration: underline; }

.error {
  color: var(--danger);
  font-size: 0.8125rem;
  margin-bottom: 12px;
  text-align: center;
}

.step-actions {
  display: flex;
  gap: 8px;
}

.step-actions button {
  flex: 1;
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
