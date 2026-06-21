<script setup lang="ts">
import type { ParsedNotes } from "../types";

defineProps<{
  lastNotes: string;
  lastParsed: ParsedNotes | null;
}>();
</script>

<template>
  <div class="card glass highlight-card">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="card-icon"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2zM22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>
      <h2>Current Active Slide</h2>
    </div>
    <div class="card-body">
      <div class="slide-notes-container">
        <label>Slide Notes</label>
        <div class="notes-box">
          {{ lastNotes || 'No notes currently received.' }}
        </div>
      </div>

      <div v-if="lastParsed" class="parsed-results">
        <div class="tag-section">
          <div class="section-label">Regular Tags (Direct Redirects)</div>
          <div class="tag-list">
            <span v-for="tag in lastParsed.regular_tags" :key="tag" class="tag badge-regular">
              #{{ tag }}
            </span>
            <span v-if="lastParsed.regular_tags.length === 0" class="tag-empty">None found</span>
          </div>
        </div>

        <div class="tag-section">
          <div class="section-label">Video Timestamps (Scheduled Redirects)</div>
          <div class="timestamp-grid">
            <div v-for="tag in lastParsed.timed_tags" :key="tag.seconds + tag.keyword" class="timestamp-pill">
              <span class="time">{{ tag.raw_time }}</span>
              <span class="word">#{{ tag.keyword }}</span>
            </div>
            <div v-if="lastParsed.timed_tags.length === 0" class="tag-empty">None found</div>
          </div>
        </div>

        <div v-if="lastParsed.invalid_tags.length > 0" class="tag-section">
          <div class="section-label text-warning">Malformed Tags</div>
          <div class="tag-list">
            <span v-for="tag in lastParsed.invalid_tags" :key="tag" class="tag badge-warning">
              {{ tag }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.slide-notes-container {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.slide-notes-container label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.notes-box {
  background: var(--notes-box-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 16px;
  font-size: 15px;
  line-height: 1.6;
  color: var(--text-color);
  min-height: 80px;
  white-space: pre-wrap;
}

.parsed-results {
  display: flex;
  flex-direction: column;
  gap: 16px;
  border-top: 1px solid var(--border-color);
  padding-top: 20px;
}

.tag-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-label {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--text-muted);
  letter-spacing: 0.5px;
  opacity: 0.8;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag {
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
}

.badge-regular {
  background: rgba(56, 189, 248, 0.12);
  color: #38bdf8;
  border: 1px solid rgba(56, 189, 248, 0.25);
}

.badge-warning {
  background: rgba(234, 179, 8, 0.12);
  color: #facc15;
  border: 1px solid rgba(234, 179, 8, 0.25);
}

.text-warning {
  color: #facc15 !important;
}

.tag-empty {
  font-size: 13px;
  color: var(--text-muted);
  font-style: italic;
  opacity: 0.5;
}

.timestamp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 8px;
}

.timestamp-pill {
  display: flex;
  background: rgba(139, 92, 246, 0.1);
  border: 1px solid rgba(139, 92, 246, 0.2);
  border-radius: 8px;
  overflow: hidden;
  font-size: 13px;
}

.timestamp-pill .time {
  background: rgba(139, 92, 246, 0.2);
  color: #c084fc;
  padding: 6px 10px;
  font-weight: 700;
  font-family: monospace;
}

.timestamp-pill .word {
  color: var(--text-color);
  padding: 6px 10px;
  font-weight: 500;
  display: flex;
  align-items: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
