<template>
  <div class="modal-overlay" @click.self="close">
    <div class="modal">
      <h2>Rejoindre un serveur</h2>

      <form @submit.prevent="handleSubmit">
        <div class="field">
          <label>Nom (pour toi)</label>
          <input v-model="name" type="text" placeholder="Mon serveur" required autofocus />
        </div>

        <div class="field">
          <label>Adresse du serveur</label>
          <input v-model="url" type="text" placeholder="http://localhost:3000" required />
        </div>

        <div class="field">
          <label>Nom d'utilisateur</label>
          <input v-model="username" type="text" placeholder="Username" required />
        </div>

        <div class="field">
          <label>Mot de passe</label>
          <input v-model="password" type="password" placeholder="Password" required />
        </div>

        <div class="field" v-if="showServerPassword">
          <label>Mot de passe serveur</label>
          <input v-model="serverPassword" type="password" placeholder="Server password" />
        </div>

        <button type="submit" :disabled="loading">
          {{ loading ? "Connexion..." : "Se connecter" }}
        </button>

        <p class="toggle" @click="showServerPassword = !showServerPassword">
          {{ showServerPassword ? "J'ai déjà un compte" : "Première connexion ?" }}
        </p>

        <p class="error" v-if="error">{{ error }}</p>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { store, addServer } from "../store";

const name = ref("");
const url = ref("http://localhost:3000");
const username = ref("");
const password = ref("");
const serverPassword = ref("");
const showServerPassword = ref(false);
const loading = ref(false);
const error = ref("");

function close() {
  store.showAddServerModal = false;
}

async function handleSubmit() {
  loading.value = true;
  error.value = "";
  try {
    await addServer(
      name.value,
      url.value,
      username.value,
      password.value,
      showServerPassword.value ? serverPassword.value : undefined
    );
    close();
  } catch (e: any) {
    if (e.message === "401") error.value = "Mot de passe incorrect";
    else if (e.message === "403") error.value = "Mot de passe serveur incorrect";
    else error.value = "Impossible de se connecter au serveur";
  } finally {
    loading.value = false;
  }
}
</script>
