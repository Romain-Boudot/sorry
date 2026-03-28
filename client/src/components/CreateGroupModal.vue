<template>
  <div class="modal-overlay" @click.self="close">
    <div class="modal-small">
      <h3>Creer un groupe</h3>
      <form @submit.prevent="submit">
        <div class="field">
          <label>Nom</label>
          <input v-model="name" type="text" placeholder="Mon groupe" required autofocus />
        </div>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="close">Annuler</button>
          <button type="submit" :disabled="!name.trim()">Creer</button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { activeState, activeServer } from "../store";
import { api } from "../api";

const emit = defineEmits<{ close: [] }>();

const name = ref("");

async function submit() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st || !name.value.trim()) return;

  const group = await api.createGroup(s.url, s.token, name.value.trim());
  st.groups.push(group);
  close();
}

function close() {
  emit("close");
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

.modal-small {
  background: var(--bg-primary);
  padding: 24px;
  border-radius: 8px;
  width: 360px;
}

.modal-small h3 {
  font-size: 1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-bottom: 16px;
}

.field {
  margin-bottom: 14px;
}

.field label {
  display: block;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.field input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}

.field input::placeholder {
  color: var(--text-faint);
}

.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 16px;
}

.modal-actions button {
  width: auto;
  padding: 8px 16px;
  font-size: 0.8125rem;
  border-radius: 6px;
}

.btn-cancel {
  background: transparent;
  color: var(--text-muted);
}

.btn-cancel:hover {
  color: var(--text-normal);
  background: transparent;
}
</style>
