const STORAGE_KEY = "sorry-emoji-freq";
const TOP_COUNT = 3;
const DEFAULTS = ["👍", "😂", "❤️"];

function load(): Record<string, number> {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
  } catch {
    return {};
  }
}

function save(freq: Record<string, number>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(freq));
}

export function recordEmoji(emoji: string) {
  const freq = load();
  freq[emoji] = (freq[emoji] ?? 0) + 1;
  save(freq);
}

export function topEmojis(): string[] {
  const freq = load();
  const sorted = Object.entries(freq)
    .sort((a, b) => b[1] - a[1])
    .slice(0, TOP_COUNT)
    .map(([emoji]) => emoji);

  if (sorted.length >= TOP_COUNT) return sorted;

  // Fill with defaults that aren't already in the list
  for (const d of DEFAULTS) {
    if (sorted.length >= TOP_COUNT) break;
    if (!sorted.includes(d)) sorted.push(d);
  }
  return sorted;
}
