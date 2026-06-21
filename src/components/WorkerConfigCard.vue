<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  baseUrl: string;
  slug: string;
  minSpacingMs: number;
}>();

const emit = defineEmits<{
  (e: "update:baseUrl", val: string): void;
  (e: "update:slug", val: string): void;
  (e: "update:minSpacingMs", val: number): void;
  (e: "save"): void;
  (e: "open"): void;
  (e: "resetPassword"): void;
}>();

const baseUrlVal = computed({
  get: () => props.baseUrl,
  set: (val) => emit("update:baseUrl", val)
});

const slugVal = computed({
  get: () => props.slug,
  set: (val) => emit("update:slug", val)
});

const minSpacingMsVal = computed({
  get: () => props.minSpacingMs,
  set: (val) => emit("update:minSpacingMs", val)
});
</script>

<template>
  <div class="card glass">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="card-icon"><path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>
      <h2>Cloudflare Worker Config</h2>
    </div>
    <div class="card-body">
      <div class="input-group">
        <label for="worker-url">Base URL</label>
        <input id="worker-url" v-model="baseUrlVal" placeholder="https://your-worker.workers.dev" />
      </div>

      <div class="input-row">
        <div class="input-group flex-grow">
          <label for="worker-slug">Active Shortlink Slug</label>
          <input id="worker-slug" v-model="slugVal" placeholder="e.g. sermon" />
        </div>
        <div class="input-group">
          <label for="spacing">Min Spacing (ms)</label>
          <input id="spacing" type="number" v-model.number="minSpacingMsVal" placeholder="1000" />
        </div>
      </div>
      <div class="card-actions button-group">
        <button class="btn btn-success" @click="emit('save')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>
          Save Worker Connection
        </button>
        <button class="btn btn-secondary" @click="emit('open')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
          Open Shortlink
        </button>
      </div>
      <div class="card-actions reset-action">
        <button class="btn btn-danger-outline" @click="emit('resetPassword')">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>
          Reset Tappy Password
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.input-row {
  display: flex;
  gap: 16px;
}

.flex-grow {
  flex-grow: 1;
}

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

.button-group {
  display: flex;
  gap: 12px;
}

.button-group .btn {
  flex: 1;
}

.reset-action {
  margin-top: 12px;
  border-top: 1px solid var(--border-color);
  padding-top: 16px;
}

.btn-danger-outline {
  width: 100%;
  background: transparent;
  color: var(--error);
  border: 1px solid var(--error);
  padding: 10px;
  border-radius: 12px;
  font-weight: 600;
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s ease;
}

.btn-danger-outline:hover {
  background: var(--error);
  color: #ffffff;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.2);
}
</style>
