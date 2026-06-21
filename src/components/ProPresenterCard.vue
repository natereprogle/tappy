<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  host: string;
  port: number;
  isConnected: boolean;
}>();

const emit = defineEmits<{
  (e: "update:host", value: string): void;
  (e: "update:port", value: number): void;
  (e: "connect"): void;
  (e: "disconnect"): void;
}>();

const hostVal = computed({
  get: () => props.host,
  set: (val) => emit("update:host", val)
});

const portVal = computed({
  get: () => props.port,
  set: (val) => emit("update:port", val)
});
</script>

<template>
  <div class="card glass">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="card-icon"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"/><rect x="2" y="14" width="20" height="8" rx="2" ry="2"/><line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/></svg>
      <h2>ProPresenter Connection</h2>
    </div>
    <div class="card-body">
      <div class="input-group">
        <label for="host">Host IP</label>
        <input id="host" v-model="hostVal" placeholder="e.g. 127.0.0.1" />
      </div>
      <div class="input-group">
        <label for="port">Port</label>
        <input id="port" type="number" v-model.number="portVal" placeholder="59066" />
      </div>
      <div class="card-actions">
        <button v-if="!isConnected" class="btn btn-primary" @click="emit('connect')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>
          Connect to ProPresenter
        </button>
        <button v-else class="btn btn-secondary danger-btn" @click="emit('disconnect')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="9" y1="9" x2="15" y2="15"/><line x1="15" y1="9" x2="9" y2="15"/></svg>
          Disconnect
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.input-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
}

.input-group label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.card-actions {
  display: flex;
}

.card-actions .btn {
  width: 100%;
}

.danger-btn {
  background: var(--btn-secondary-bg) !important;
  color: var(--text-color) !important;
}

.danger-btn:hover {
  background: var(--error) !important;
  color: #ffffff !important;
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
}
</style>
