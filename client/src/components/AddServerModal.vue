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
        <div class="server-header">
          <div class="server-header-icon">
            <img v-if="serverIconUrl" :src="serverIconUrl" />
            <span v-else>{{ serverName[0]?.toUpperCase() }}</span>
          </div>
          <div>
            <h2>{{ serverName }}</h2>
            <p v-if="serverDescription" class="server-header-desc">{{ serverDescription }}</p>
          </div>
        </div>

        <!-- Guest invite detected → display name only -->
        <template v-if="guestMode">
          <p class="step-desc">Cette invitation ne necessite pas de compte. Choisis juste un pseudo pour rejoindre.</p>

          <div class="field">
            <label>Pseudo</label>
            <input v-model="displayName" type="text" placeholder="Ton pseudo" required autofocus @keydown.enter="nextStep" />
          </div>

          <p class="error" v-if="error">{{ error }}</p>

          <div class="step-actions">
            <button class="btn-back" @click="guestMode = false; showNewAccount = false; inviteCode = ''">Retour</button>
            <button @click="nextStep" :disabled="loading || !displayName.trim()">
              {{ loading ? "Connexion..." : "Rejoindre" }}
            </button>
          </div>
        </template>

        <!-- New account (non-guest invite) -->
        <template v-else-if="showNewAccount">
          <template v-if="!inviteChecked">
            <p class="step-desc">Entre ton code d'invitation pour rejoindre le serveur.</p>

            <div class="field">
              <label>Code d'invitation</label>
              <input v-model="inviteCode" type="text" placeholder="Demande a l'admin" required autofocus @keydown.enter="checkInviteCode" />
            </div>

            <p class="toggle" @click="showNewAccount = false">J'ai deja un compte</p>
            <p class="error" v-if="error">{{ error }}</p>

            <div class="step-actions">
              <button class="btn-back" @click="step = 1">Retour</button>
              <button @click="checkInviteCode" :disabled="loading || !inviteCode.trim()">
                {{ loading ? "Verification..." : "Suivant" }}
              </button>
            </div>
          </template>

          <template v-else>
            <p class="step-desc">Ton identifiant est prive et sert uniquement a te connecter. Les autres verront ton display name.</p>

            <div class="field-group">
              <div class="field">
                <label>Identifiant</label>
                <input v-model="username" type="text" placeholder="ton_id" required autofocus />
              </div>
              <div class="field">
                <label>Mot de passe</label>
                <input v-model="password" type="password" placeholder="********" required @keydown.enter="nextStep" />
              </div>
            </div>

            <div class="field">
              <label>Display name</label>
              <input v-model="displayName" type="text" placeholder="Ton nom visible" />
            </div>

            <p class="error" v-if="error">{{ error }}</p>

            <div class="step-actions">
              <button class="btn-back" @click="inviteChecked = false">Retour</button>
              <button @click="nextStep" :disabled="loading || !username.trim() || !password.trim()">
                {{ loading ? "Inscription..." : "S'inscrire" }}
              </button>
            </div>
          </template>
        </template>

        <!-- Login (existing account) -->
        <template v-else>
          <p class="step-desc">Connecte-toi avec ton identifiant et mot de passe.</p>

          <div class="field-group">
            <div class="field">
              <label>Identifiant</label>
              <input v-model="username" type="text" placeholder="ton_id" required autofocus />
            </div>
            <div class="field">
              <label>Mot de passe</label>
              <input v-model="password" type="password" placeholder="********" required @keydown.enter="nextStep" />
            </div>
          </div>

          <template v-if="needsTotp">
            <div class="field">
              <label>Code TOTP</label>
              <input v-model="totpCode" type="text" inputmode="numeric" pattern="[0-9]*" maxlength="6" placeholder="000000" autofocus @keydown.enter="nextStep" />
            </div>
          </template>

          <p v-if="!needsTotp" class="toggle" @click="showNewAccount = true">Premiere connexion ?</p>

          <p class="error" v-if="error">{{ error }}</p>

          <div class="step-actions">
            <button class="btn-back" @click="needsTotp ? (needsTotp = false) : (step = 1)">Retour</button>
            <button @click="nextStep" :disabled="loading || !username.trim() || !password.trim() || (needsTotp && totpCode.length < 6)">
              {{ loading ? "Connexion..." : "Se connecter" }}
            </button>
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { store, addServer, addServerGuest } from "../store";
import { api, resolveBaseUrl } from "../api";

const step = ref(1);
const loading = ref(false);
const error = ref("");

// Step 1
const url = ref("");
const serverName = ref("");

// Step 2
const username = ref("");
const password = ref("");
const displayName = ref(localStorage.getItem("defaultDisplayName") || "");
const inviteCode = ref("");
const showNewAccount = ref(false);
const guestMode = ref(false);
const inviteChecked = ref(false);
const needsTotp = ref(false);
const totpCode = ref("");
const serverIconUrl = ref<string | null>(null);
const serverDescription = ref<string | null>(null);

// Handle prefilled invite link
onMounted(async () => {
  if (store.prefillServerUrl && store.prefillInviteCode) {
    url.value = store.prefillServerUrl;
    inviteCode.value = store.prefillInviteCode;
    showNewAccount.value = true;
    const prefillUrl = store.prefillServerUrl;
    store.prefillServerUrl = "";
    store.prefillInviteCode = "";
    // Auto-resolve and skip to step 2, then auto-check invite
    loading.value = true;
    try {
      const resolved = await resolveBaseUrl(prefillUrl);
      url.value = resolved;
      const info = await api.serverInfo(resolved);
      serverName.value = info.name;
      serverIconUrl.value = info.icon_url ? `${resolved}${info.icon_url}` : null;
      serverDescription.value = info.description ?? null;
      step.value = 2;
      // Auto-check the invite
      await checkInviteCode();
    } catch {
      error.value = "Impossible de joindre ce serveur";
    } finally {
      loading.value = false;
    }
  }
});

async function checkInviteCode() {
  error.value = "";
  loading.value = true;
  try {
    const result = await api.checkInvite(url.value, inviteCode.value);
    if (result.guest) {
      guestMode.value = true;
    } else {
      inviteChecked.value = true;
    }
  } catch (e: any) {
    if (e.message === "404") {
      error.value = "Code d'invitation invalide";
    } else if (e.message === "410") {
      error.value = "Cette invitation a expire ou atteint sa limite";
    } else if (e.message === "429") {
      error.value = "Trop de tentatives, reessaie dans une minute";
    } else {
      error.value = "Impossible de verifier l'invitation";
    }
  } finally {
    loading.value = false;
  }
}

async function nextStep() {
  error.value = "";
  loading.value = true;

  try {
    if (step.value === 1) {
      const resolved = await resolveBaseUrl(url.value);
      url.value = resolved;
      const info = await api.serverInfo(resolved);
      serverName.value = info.name;
      serverIconUrl.value = info.icon_url ? `${url.value}${info.icon_url}` : null;
      serverDescription.value = info.description ?? null;
      step.value = 2;
    } else if (step.value === 2 && guestMode.value) {
      await addServerGuest(
        serverName.value,
        url.value,
        inviteCode.value,
        displayName.value.trim(),
      );
      close();
    } else if (step.value === 2) {
      // Reconstruct default avatar file from localStorage if available
      let defaultAvatar: File | undefined;
      if (showNewAccount.value) {
        const avatarData = localStorage.getItem("defaultAvatarPreview");
        const avatarName = localStorage.getItem("defaultAvatarName");
        const avatarType = localStorage.getItem("defaultAvatarType");
        if (avatarData && avatarName && avatarType) {
          try {
            const res = await fetch(avatarData);
            const blob = await res.blob();
            defaultAvatar = new File([blob], avatarName, { type: avatarType });
          } catch {}
        }
      }
      await addServer(
        serverName.value,
        url.value,
        username.value,
        password.value,
        showNewAccount.value ? inviteCode.value : undefined,
        showNewAccount.value && displayName.value.trim() ? displayName.value.trim() : undefined,
        defaultAvatar,
        needsTotp.value ? totpCode.value : undefined
      );
      close();
    }
  } catch (e: any) {
    if (step.value === 1) {
      error.value = "Impossible de joindre ce serveur";
    } else if (e.message === "totp_required") {
      needsTotp.value = true;
      error.value = "";
    } else if (e.message === "401") {
      error.value = needsTotp.value ? "Code TOTP incorrect" : "Mot de passe incorrect";
      totpCode.value = "";
    } else if (e.message === "429") {
      error.value = "Trop de tentatives, reessaie dans une minute";
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
  gap: 14px;
  margin-bottom: 12px;
}

.server-header-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  font-weight: 700;
  font-size: 1.25rem;
  color: var(--text-muted);
}

.server-header-icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.server-header h2 {
  margin-bottom: 0;
}

.server-header-desc {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-top: 2px;
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
