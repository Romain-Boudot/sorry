<template>
  <ModalSmall title="Modifier le channel" @close="close">
    <form @submit.prevent="submit">
      <div class="field">
        <label>Nom</label>
        <input ref="inputRef" v-model="name" type="text" required />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn-cancel" @click="close">Annuler</button>
        <button type="submit" :disabled="!name.trim() || name.trim() === props.channel.name">Enregistrer</button>
      </div>
    </form>
  </ModalSmall>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { activeState, activeServer } from "../store";
import { api, type Channel } from "../api";
import ModalSmall from "./ModalSmall.vue";

const props = defineProps<{ channel: Channel }>();
const emit = defineEmits<{ close: [] }>();

const name = ref(props.channel.name);
const inputRef = ref<HTMLInputElement | null>(null);

onMounted(() => inputRef.value?.focus());

async function submit() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st || !name.value.trim()) return;

  const updated = await api.updateChannel(s.url, s.token, props.channel.id, { name: name.value.trim() });
  const idx = st.channels.findIndex((c) => c.id === props.channel.id);
  if (idx >= 0) st.channels[idx] = updated;
  close();
}

function close() {
  emit("close");
}
</script>
