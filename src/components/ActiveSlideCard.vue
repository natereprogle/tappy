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
.card {
  transition: transform 0.2s ease;
}

.slide-notes-container {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.slide-notes-container label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.notes-box {
  background: var(--notes-box-bg);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  padding: 20px;
  font-size: 16px;
  line-height: 1.6;
  color: var(--text-color);
  min-height: 100px;
  white-space: pre-wrap;
  box-shadow: inset 0 2px 4px rgba(0,0,0,0.02);
}

.parsed-results {
  display: flex;
  flex-direction: column;
  gap: 20px;
  border-top: 1px solid var(--border-color);
  padding-top: 24px;
}

.tag-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-label {
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--text-muted);
  letter-spacing: 1px;
  opacity: 0.8;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.tag {
  padding: 6px 14px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 700;
  transition: all 0.2s ease;
}

.badge-regular {
  background: rgba(99, 102, 241, 0.1);
  color: var(--primary-color);
  border: 1px solid rgba(99, 102, 241, 0.2);
}

.badge-warning {
  background: rgba(245, 158, 11, 0.1);
  color: var(--warning);
  border: 1px solid rgba(245, 158, 11, 0.2);
}

.text-warning {
  color: var(--warning) !important;
}

.tag-empty {
  font-size: 13px;
  color: var(--text-muted);
  font-style: italic;
  opacity: 0.5;
}

.timestamp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
}

.timestamp-pill {
  display: flex;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
  font-size: 13px;
  transition: all 0.2s ease;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

.timestamp-pill:hover {
  border-color: var(--primary-color);
  transform: translateY(-1px);
}

.timestamp-pill .time {
  background: var(--primary-color);
  color: white;
  padding: 8px 12px;
  font-weight: 800;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
}

.timestamp-pill .word {
  color: var(--text-color);
  padding: 8px 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
