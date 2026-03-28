<template>
  <ModalSmall title="Creer un groupe" @close="close">
    <form @submit.prevent="submit">
      <div class="field">
        <label>Nom</label>
        <input ref="inputRef" v-model="name" type="text" placeholder="Mon groupe" required />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn-cancel" @click="close">Annuler</button>
        <button type="submit" :disabled="!name.trim()">Creer</button>
      </div>
    </form>
  </ModalSmall>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { activeState, activeServer } from "../store";
import { api } from "../api";
import ModalSmall from "./ModalSmall.vue";

const emit = defineEmits<{ close: [] }>();

const name = ref("");
const inputRef = ref<HTMLInputElement | null>(null);

onMounted(() => inputRef.value?.focus());

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
