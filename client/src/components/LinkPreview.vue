<template>
  <!-- YouTube embed -->
  <div v-if="youtubeId" class="embed-container">
    <iframe
      :src="`https://www.youtube.com/embed/${youtubeId}`"
      frameborder="0"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
      allowfullscreen
    ></iframe>
  </div>

  <!-- Spotify embed -->
  <div v-else-if="spotifyEmbed" class="embed-container spotify">
    <iframe
      :src="spotifyEmbed"
      frameborder="0"
      allow="encrypted-media"
      allowfullscreen
    ></iframe>
  </div>

  <!-- Generic OG preview -->
  <a v-else-if="og" class="link-preview" :href="og.url" target="_blank" rel="noopener noreferrer">
    <img v-if="og.image" :src="og.image" class="link-preview-image" loading="lazy" @error="og!.image = undefined" />
    <div class="link-preview-body">
      <span v-if="og.site_name" class="link-preview-site">{{ og.site_name }}</span>
      <span v-if="og.title" class="link-preview-title">{{ og.title }}</span>
      <span v-if="og.description" class="link-preview-desc">{{ og.description }}</span>
    </div>
  </a>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { activeServer } from "../store";
import { api } from "../api";

const props = defineProps<{ url: string }>();

// ── YouTube detection ──
const youtubeId = computed(() => {
  const m =
    props.url.match(/youtube\.com\/watch\?v=([^&\s]+)/) ||
    props.url.match(/youtu\.be\/([^?\s]+)/) ||
    props.url.match(/youtube\.com\/shorts\/([^?\s]+)/);
  return m?.[1] ?? null;
});

// ── Spotify detection ──
const spotifyEmbed = computed(() => {
  // open.spotify.com/track/xxx → embed url
  const m = props.url.match(/open\.spotify\.com\/(track|album|playlist|episode|show)\/([^?\s]+)/);
  if (!m) return null;
  return `https://open.spotify.com/embed/${m[1]}/${m[2]}`;
});

// ── Generic OG fallback ──
const og = ref<{
  title?: string;
  description?: string;
  image?: string;
  site_name?: string;
  url: string;
} | null>(null);

onMounted(async () => {
  // Skip OG fetch for embedded providers
  if (youtubeId.value || spotifyEmbed.value) return;

  const server = activeServer();
  if (!server) return;
  try {
    og.value = await api.fetchOg(server.url, server.token, props.url);
    if (!og.value.title && !og.value.description) {
      og.value = null;
    }
  } catch {
    og.value = null;
  }
});
</script>

<style scoped>
.embed-container {
  margin-top: 6px;
  max-width: 420px;
  border-radius: 8px;
  overflow: hidden;
}

.embed-container iframe {
  width: 420px;
  height: 236px;
  border: none;
}

.embed-container.spotify iframe {
  width: 420px;
  height: 152px;
}

.link-preview {
  display: flex;
  border-left: 3px solid var(--accent);
  border-radius: 4px;
  background: var(--bg-secondary);
  overflow: hidden;
  margin-top: 6px;
  max-width: 420px;
  text-decoration: none;
  color: inherit;
  transition: background 0.1s;
}

.link-preview:hover {
  background: var(--bg-modifier-hover);
}

.link-preview-image {
  width: 80px;
  min-height: 80px;
  object-fit: cover;
  flex-shrink: 0;
}

.link-preview-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  overflow: hidden;
  min-width: 0;
}

.link-preview-site {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.link-preview-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.link-preview-desc {
  font-size: 0.75rem;
  color: var(--text-muted);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
