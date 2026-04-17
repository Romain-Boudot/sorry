<template>
  <div class="settings-body">
    <div class="card">
      <div class="card-title">Avatar</div>
      <p class="card-hint">Ta photo de profil sur ce serveur.</p>
      <div class="avatar-setting">
        <div class="avatar-preview" @click="serverAvatarInput?.click()">
          <img v-if="currentAvatarUrl" :src="currentAvatarUrl" />
          <span v-else class="avatar-placeholder">{{ (state?.user?.display_name || '?')[0]?.toUpperCase() }}</span>
          <div class="avatar-overlay">
            <Camera :size="16" />
          </div>
        </div>
        <div class="avatar-actions">
          <button class="btn-sm" @click="serverAvatarInput?.click()">Changer</button>
          <button v-if="currentAvatarUrl" class="btn-sm btn-danger-outline" @click="removeServerAvatar">Supprimer</button>
        </div>
        <input ref="serverAvatarInput" type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden @change="onServerAvatarSelect" />
      </div>
    </div>

    <div class="card">
      <div class="card-title">Display name</div>
      <p class="card-hint">Ton nom visible sur ce serveur.</p>
      <div class="input-row">
        <BaseInput v-model="displayName" placeholder="Mon pseudo" :maxlength="32" />
        <SaveButton :loading="saving" :saved="saved" @click="saveDisplayName" />
      </div>
    </div>

    <div class="card">
      <div class="card-title">Mot de passe</div>
      <p class="card-hint">Change ton mot de passe de connexion.</p>
      <div class="password-fields">
        <BaseInput v-model="currentPassword" type="password" placeholder="Mot de passe actuel" />
        <BaseInput v-model="newPassword" type="password" placeholder="Nouveau mot de passe" />
        <BaseInput v-model="confirmPassword" type="password" placeholder="Confirmer" @keydown.enter="changePassword" />
      </div>
      <div class="password-actions">
        <SaveButton label="Changer" :loading="savingPassword" :saved="passwordSaved" :disabled="!currentPassword || !newPassword || newPassword !== confirmPassword" @click="changePassword" />
        <span v-if="newPassword && confirmPassword && newPassword !== confirmPassword" class="password-error">Les mots de passe ne correspondent pas</span>
        <span v-if="passwordError" class="password-error">{{ passwordError }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { Camera } from "lucide-vue-next";
import SaveButton from "../ui/SaveButton.vue";
import BaseInput from "../ui/BaseInput.vue";
import { activeState, activeServer } from "../../store";
import { api } from "../../api";

const state = computed(() => activeState());

const displayName = ref(state.value?.user?.display_name || "");
const saving = ref(false);
const saved = ref(false);
const serverAvatarInput = ref<HTMLInputElement>();

const currentAvatarUrl = computed(() => {
  const s = activeServer();
  const avatarPath = state.value?.user?.avatar_url;
  if (!s || !avatarPath) return null;
  return `${s.url}${avatarPath}`;
});

async function onServerAvatarSelect(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  try {
    const user = await api.uploadAvatar(s.url, s.token, file);
    st.user = user;
    st.users.set(user.id, user);
  } catch {}
  input.value = "";
}

async function removeServerAvatar() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  try {
    await api.deleteAvatar(s.url, s.token);
    if (st.user) {
      st.user.avatar_url = null;
      st.users.set(st.user.id, { ...st.user });
    }
  } catch {}
}

// Password
const currentPassword = ref("");
const newPassword = ref("");
const confirmPassword = ref("");
const savingPassword = ref(false);
const passwordError = ref("");
const passwordSaved = ref(false);

async function changePassword() {
  const s = activeServer();
  if (!s || !currentPassword.value || !newPassword.value) return;
  if (newPassword.value !== confirmPassword.value) return;

  savingPassword.value = true;
  passwordError.value = "";
  passwordSaved.value = false;
  try {
    await api.changePassword(s.url, s.token, currentPassword.value, newPassword.value);
    passwordSaved.value = true;
    currentPassword.value = "";
    newPassword.value = "";
    confirmPassword.value = "";
    setTimeout(() => (passwordSaved.value = false), 3000);
  } catch (e: any) {
    if (e.message === "401") {
      passwordError.value = "Mot de passe actuel incorrect";
    } else {
      passwordError.value = "Erreur";
    }
  } finally {
    savingPassword.value = false;
  }
}

async function saveDisplayName() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  saving.value = true;
  saved.value = false;
  try {
    const user = await api.updateDisplayName(s.url, s.token, displayName.value.trim());
    st.user = user;
    st.users.set(user.id, user);
    saved.value = true;
    setTimeout(() => (saved.value = false), 2000);
  } finally {
    saving.value = false;
  }
}
</script>

<style scoped>
.settings-body {
  padding: 0 24px 24px;
  overflow-y: auto;
  flex: 1;
}

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

.avatar-setting {
  display: flex;
  align-items: center;
  gap: 16px;
}

.avatar-preview {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  position: relative;
  overflow: hidden;
  flex-shrink: 0;
}

.avatar-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.avatar-placeholder {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-faint);
}

.avatar-overlay {
  position: absolute;
  inset: 0;
  background: var(--overlay-light);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-bright);
  opacity: 0;
  transition: opacity 0.15s;
}

.avatar-preview:hover .avatar-overlay {
  opacity: 1;
}

.avatar-actions {
  display: flex;
  gap: 8px;
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

.password-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 10px;
}

.password-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.password-error {
  font-size: 0.75rem;
  color: var(--danger);
}
</style>
