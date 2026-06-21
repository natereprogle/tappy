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
  min-height: 280px;
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
  background: var(--btn-secondary-bg);
  border: none;
  color: var(--text-muted);
  font-size: 11px;
  cursor: pointer;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 6px 12px;
  border-radius: 8px;
  transition: all 0.2s ease;
}

.btn-clear:hover {
  color: var(--error);
  background: rgba(239, 68, 68, 0.1);
}

.log-container {
  overflow-y: auto;
  max-height: 400px;
  background: var(--log-container-bg);
  border-radius: 16px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border: 1px solid var(--border-color);
  box-shadow: inset 0 2px 4px rgba(0,0,0,0.02);
}

/* Custom Scrollbar */
.log-container::-webkit-scrollbar {
  width: 6px;
}

.log-container::-webkit-scrollbar-track {
  background: transparent;
}

.log-container::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 10px;
}

.log-container::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}

.log-entry {
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  line-height: 1.6;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 10px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  box-shadow: 0 1px 2px rgba(0,0,0,0.02);
  transition: transform 0.2s ease;
}

.log-entry:hover {
  transform: translateX(4px);
  border-color: var(--primary-color);
}

.log-time {
  color: var(--text-muted);
  font-weight: 600;
  flex-shrink: 0;
  font-size: 11px;
}

.log-badge {
  background: var(--primary-color);
  color: white;
  padding: 0 6px;
  border-radius: 6px;
  font-size: 10px;
  font-weight: 800;
  text-transform: uppercase;
  flex-shrink: 0;
}

.log-msg {
  word-break: break-word;
  color: var(--text-color);
}

.log-entry.info {
  border-left: 4px solid var(--text-muted);
}

.log-entry.success {
  border-left: 4px solid var(--success);
  background: rgba(16, 185, 129, 0.02);
}

.log-entry.warning {
  border-left: 4px solid var(--warning);
  background: rgba(245, 158, 11, 0.02);
}

.log-entry.error {
  border-left: 4px solid var(--error);
  background: rgba(239, 68, 68, 0.02);
}

.log-entry.event {
  border-left: 4px solid var(--info);
  background: rgba(59, 130, 246, 0.02);
}

.log-empty-state {
  text-align: center;
  color: var(--text-muted);
  font-size: 14px;
  padding: 60px 20px;
  font-style: italic;
  opacity: 0.6;
}
</style>
