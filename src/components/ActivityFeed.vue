<script setup lang="ts">
import type { LogEntry } from "../types";

defineProps<{
  logs: LogEntry[];
}>();

const emit = defineEmits<{
  (e: "clear"): void;
}>();
</script>

<template>
  <div class="card glass log-card flex-grow">
    <div class="card-header justify-between">
      <div class="flex items-center">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="card-icon"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
        <h2>Activity Feed</h2>
      </div>
      <button class="btn-clear" @click="emit('clear')">Clear Feed</button>
    </div>
    <div class="card-body log-container">
      <div v-for="log in logs" :key="log.id" :class="['log-entry', log.type]">
        <span class="log-time">[{{ log.timestamp }}]</span>
        <span class="log-badge" v-if="log.tag">#{{ log.tag }}</span>
        <span class="log-msg">{{ log.message }}</span>
      </div>
      <div v-if="logs.length === 0" class="log-empty-state">
        Logs and activity will stream here in real-time.
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-card {
  min-height: 240px;
}

.justify-between {
  justify-content: space-between;
}

.items-center {
  align-items: center;
}

.flex {
  display: flex;
}

.btn-clear {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  font-weight: 500;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.btn-clear:hover {
  color: #f87171;
  background: rgba(239, 68, 68, 0.08);
}

.log-container {
  overflow-y: auto;
  max-height: 350px;
  background: var(--log-container-bg);
  border-radius: 12px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  border: 1px solid var(--border-color);
}

/* Custom Scrollbar for Logs */
.log-container::-webkit-scrollbar {
  width: 6px;
}

.log-container::-webkit-scrollbar-track {
  background: transparent;
}

.log-container::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

.log-container::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

.log-entry {
  font-family: monospace;
  font-size: 12px;
  line-height: 1.5;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 4px;
}

.log-time {
  color: var(--text-muted);
  opacity: 0.7;
  flex-shrink: 0;
}

.log-badge {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-color);
  padding: 0 4px;
  border-radius: 3px;
  font-size: 11px;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
}

.log-msg {
  word-break: break-word;
}

.log-entry.info {
  color: var(--text-muted);
}

.log-entry.success {
  color: #34d399;
  background: rgba(16, 185, 129, 0.05);
}

.log-entry.warning {
  color: #fbbf24;
  background: rgba(245, 158, 11, 0.05);
}

.log-entry.error {
  color: #f87171;
  background: rgba(239, 68, 68, 0.05);
}

.log-entry.event {
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.05);
}

.log-empty-state {
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
  padding: 40px 0;
  font-style: italic;
  opacity: 0.6;
}
</style>
