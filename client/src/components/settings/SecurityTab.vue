<template>
  <SettingsBody>
    <div class="card">
      <div class="card-title">Authentification a deux facteurs (TOTP)</div>
      <p class="card-hint">Ajoute une couche de securite a ton compte sur ce serveur. Compatible Google Authenticator, Authy, etc.</p>

      <template v-if="totpEnabled">
        <p class="totp-status totp-enabled">TOTP actif</p>
        <button class="btn-sm btn-danger-outline" @click="disableTotp">Desactiver le TOTP</button>
      </template>

      <template v-else-if="totpSetupData">
        <p class="card-hint">Scanne ce QR code avec ton app d'authentification, puis entre le code a 6 chiffres pour confirmer.</p>
        <div class="totp-qr">
          <img :src="totpQrUrl" alt="QR Code TOTP" />
        </div>
        <p class="totp-secret">Cle manuelle : <code>{{ totpSetupData.secret }}</code></p>
        <div class="input-row">
          <input v-model="totpConfirmCode" type="text" inputmode="numeric" pattern="[0-9]*" maxlength="6" placeholder="000000" @keydown.enter="confirmTotp" />
          <button class="btn-sm" :disabled="totpConfirmCode.length < 6" @click="confirmTotp">Verifier</button>
        </div>
        <p class="totp-error" v-if="totpError">{{ totpError }}</p>
      </template>

      <template v-else>
        <button class="btn-sm" @click="setupTotp">Activer le TOTP</button>
      </template>
    </div>
  </SettingsBody>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { activeServer } from "../../store";
import SettingsBody from "../ui/SettingsBody.vue";
import { api } from "../../api";

const totpEnabled = ref(false);
const totpSetupData = ref<{ secret: string; otpauth_url: string } | null>(null);
const totpConfirmCode = ref("");
const totpError = ref("");

const totpQrUrl = computed(() => {
  if (!totpSetupData.value) return "";
  return `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(totpSetupData.value.otpauth_url)}`;
});

onMounted(() => {
  loadTotpStatus();
});

async function loadTotpStatus() {
  const server = activeServer();
  if (!server) return;
  try {
    const res = await api.totpStatus(server.url, server.token);
    totpEnabled.value = res.enabled;
  } catch {}
}

async function setupTotp() {
  const server = activeServer();
  if (!server) return;
  totpError.value = "";
  try {
    const res = await api.totpSetup(server.url, server.token);
    totpSetupData.value = res;
  } catch {
    totpError.value = "Erreur lors de la configuration TOTP";
  }
}

async function confirmTotp() {
  const server = activeServer();
  if (!server) return;
  totpError.value = "";
  try {
    await api.totpVerify(server.url, server.token, totpConfirmCode.value);
    totpEnabled.value = true;
    totpSetupData.value = null;
    totpConfirmCode.value = "";
  } catch {
    totpError.value = "Code incorrect, reessaie";
    totpConfirmCode.value = "";
  }
}

async function disableTotp() {
  const server = activeServer();
  if (!server) return;
  try {
    await api.totpDisable(server.url, server.token);
    totpEnabled.value = false;
  } catch {}
}
</script>

<style scoped>
.card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 16px;
}

.card-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.card-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 8px;
}

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.btn-danger-outline {
  background: transparent !important;
  color: var(--danger) !important;
  border: 1px solid var(--danger) !important;
}

.btn-danger-outline:hover {
  background: var(--danger-bg-hover) !important;
  box-shadow: none !important;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row input[type="text"] {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}
.input-row input[type="text"]::placeholder { color: var(--text-faint); }

.totp-status {
  font-size: 0.875rem;
  font-weight: 600;
  margin-bottom: 12px;
}

.totp-enabled {
  color: var(--green);
}

.totp-qr {
  display: flex;
  justify-content: center;
  margin: 16px 0;
}

.totp-qr img {
  width: 200px;
  height: 200px;
  border-radius: 8px;
  background: #fff;
  padding: 8px;
}

.totp-secret {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 12px;
  text-align: center;
  word-break: break-all;
}

.totp-secret code {
  font-family: monospace;
  color: var(--text-muted);
  user-select: all;
}

.totp-error {
  color: var(--danger);
  font-size: 0.8125rem;
  margin-top: 8px;
}
</style>
