<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { AppSettings, WorkerConfig, SlideUpdate, LogEntry, ParsedNotes } from "./types";
import Onboarding from "./components/Onboarding.vue";
import ProPresenterCard from "./components/ProPresenterCard.vue";
import WorkerConfigCard from "./components/WorkerConfigCard.vue";
import TagMappingCard from "./components/TagMappingCard.vue";
import ActiveSlideCard from "./components/ActiveSlideCard.vue";
import ActivityFeed from "./components/ActivityFeed.vue";

// Navigation / Mode
const isOnboardingComplete = ref(false);

// Config States
const propresenterHost = ref("127.0.0.1");
const propresenterPort = ref(59066);
const propresenterEndpoint = ref("v1/status/slide");
const workerBaseUrl = ref("");
const workerAdminToken = ref("");
const workerSlug = ref("");
const minSpacingMs = ref(1000);
const defaultRedirectUrl = ref("https://");
const linkOwnerId = ref("tappy_user");
const tagMappings = ref<Record<string, string>>({});

const originalTagMappings = ref("");
const hasUnsavedMappings = computed(() => {
  return JSON.stringify(tagMappings.value) !== originalTagMappings.value;
});

// Rotate Secret States
const showRotateModal = ref(false);
const rotateAccountId = ref("");
const rotateApiToken = ref("");
const rotating = ref(false);
const rotateError = ref("");
const rotateSuccess = ref(false);

// Status States
const isProPresenterConnected = ref(false);
const lastNotes = ref("");
const lastParsed = ref<ParsedNotes | null>(null);
const lastMediaStarted = ref(false);
const logs = ref<LogEntry[]>([]);

function addLog(type: LogEntry["type"], message: string, tag?: string) {
  const timestamp = new Date().toLocaleTimeString();
  const id = Math.random().toString(36).substring(2, 9);
  logs.value.unshift({ id, timestamp, type, message, tag });
  if (logs.value.length > 50) {
    logs.value.pop();
  }
}

async function loadConfig() {
  try {
    const settings = await invoke<AppSettings>("load_settings");
    propresenterHost.value = settings.propresenter_host;
    propresenterPort.value = settings.propresenter_port;
    propresenterEndpoint.value = settings.propresenter_endpoint;
    workerBaseUrl.value = settings.worker_base_url;
    workerSlug.value = settings.worker_slug;
    minSpacingMs.value = settings.minimum_kv_write_spacing_ms;
    
    // Loaded new fields
    defaultRedirectUrl.value = settings.default_redirect_url || "https://";
    linkOwnerId.value = settings.link_owner_id || "tappy_user";
    tagMappings.value = settings.tag_mappings || {};
    originalTagMappings.value = JSON.stringify(settings.tag_mappings || {});

    if (workerBaseUrl.value) {
      isOnboardingComplete.value = true;
      addLog("info", "Settings loaded from disk");
    } else {
      isOnboardingComplete.value = false;
      addLog("info", "No settings found. Entering onboarding flow...");
    }

    // Also see if worker has config already
    const workerConfig = await invoke<WorkerConfig | null>("get_worker_config");
    if (workerConfig) {
      workerAdminToken.value = workerConfig.admin_token;
    }
  } catch (err) {
    addLog("error", `Failed to load settings: ${err}`);
  }
}

async function saveConfig() {
  try {
    const settings: AppSettings = {
      propresenter_host: propresenterHost.value,
      propresenter_port: propresenterPort.value,
      propresenter_endpoint: propresenterEndpoint.value,
      worker_base_url: workerBaseUrl.value,
      worker_slug: workerSlug.value,
      minimum_kv_write_spacing_ms: minSpacingMs.value,
      default_redirect_url: defaultRedirectUrl.value,
      link_owner_id: linkOwnerId.value,
      tag_mappings: tagMappings.value,
    };

    await invoke("save_settings", { settings });
    
    // Configure worker config on backend
    await invoke("configure_worker", {
      baseUrl: workerBaseUrl.value,
      adminToken: workerAdminToken.value,
      slug: workerSlug.value,
    });

    addLog("success", "Settings and Worker Config saved & synchronized");
  } catch (err) {
    addLog("error", `Failed to save settings: ${err}`);
  }
}

async function handleOnboardingComplete(config: {
  baseUrl: string;
  adminToken: string;
  slug: string;
  defaultUrl: string;
  ownerId: string;
}) {
  workerBaseUrl.value = config.baseUrl;
  workerAdminToken.value = config.adminToken;
  workerSlug.value = config.slug;
  defaultRedirectUrl.value = config.defaultUrl;
  linkOwnerId.value = config.ownerId;
  isOnboardingComplete.value = true;

  await saveConfig();
  addLog("success", "Onboarding completed successfully!");
}

async function pushTagMappings() {
  try {
    addLog("info", "Pushing tag mappings to Cloudflare Worker...");
    await invoke("create_or_update_master_link", {
      defaultUrl: defaultRedirectUrl.value,
      ownerId: linkOwnerId.value,
      tags: tagMappings.value,
    });
    
    // Make sure we also save them locally
    await saveConfig();
    originalTagMappings.value = JSON.stringify(tagMappings.value);
    addLog("success", `Successfully pushed ${Object.keys(tagMappings.value).length} tag mappings!`);
  } catch (err) {
    addLog("error", `Failed to push tag mappings: ${err}`);
  }
}

async function connectToProPresenter() {
  try {
    addLog("info", `Attempting connection to ProPresenter at ${propresenterHost.value}:${propresenterPort.value}...`);
    await invoke("start_propresenter_listener", {
      host: propresenterHost.value,
      port: propresenterPort.value,
      endpoint: propresenterEndpoint.value,
    });
  } catch (err) {
    addLog("error", `Failed to trigger ProPresenter listener: ${err}`);
  }
}

async function disconnectFromProPresenter() {
  try {
    addLog("info", "Disconnecting from ProPresenter...");
    await invoke("stop_propresenter_listener");
  } catch (err) {
    addLog("error", `Failed to disconnect from ProPresenter: ${err}`);
  }
}

async function openShortlink() {
  try {
    await invoke("open_current_shortlink");
    addLog("info", "Requested system browser to open Cloudflare shortlink");
  } catch (err) {
    addLog("error", `Failed to open shortlink: ${err}`);
  }
}

function closeRotateModal() {
  showRotateModal.value = false;
  rotateAccountId.value = "";
  rotateApiToken.value = "";
  rotateError.value = "";
  rotateSuccess.value = false;
}

async function confirmRotatePassword() {
  if (!rotateAccountId.value.trim() || !rotateApiToken.value.trim()) {
    rotateError.value = "Account ID and API Token are required.";
    return;
  }

  rotating.value = true;
  rotateError.value = "";
  rotateSuccess.value = false;
  addLog("info", "Starting password rotation sequence...");

  try {
    // 1. Invoke rotation on Cloudflare & local backend AppState
    const result = await invoke<{ admin_token: string }>("rotate_worker_token", {
      accountId: rotateAccountId.value.trim(),
      apiToken: rotateApiToken.value.trim(),
    });

    workerAdminToken.value = result.admin_token;
    addLog("success", "Worker admin token successfully rotated on Cloudflare!");

    // 2. Save settings locally (saves configure_worker and saves settings.json)
    await saveConfig();

    // 3. Push existing tag mappings to synchronize (with retries for propagation delay)
    addLog("info", "Synchronizing tag mappings with new admin token...");
    
    let synced = false;
    let attempts = 0;
    const maxAttempts = 6;
    while (!synced && attempts < maxAttempts) {
      try {
        attempts++;
        if (attempts > 1) {
          addLog("info", `Retrying synchronization (attempt ${attempts}/${maxAttempts})...`);
        }
        await invoke("create_or_update_master_link", {
          defaultUrl: defaultRedirectUrl.value,
          ownerId: linkOwnerId.value,
          tags: tagMappings.value,
        });
        synced = true;
      } catch (err) {
        if (attempts >= maxAttempts) {
          throw err;
        }
        // Wait 1.5 seconds before retrying
        await new Promise((resolve) => setTimeout(resolve, 1500));
      }
    }

    originalTagMappings.value = JSON.stringify(tagMappings.value);
    addLog("success", "Tag mappings successfully synchronized with new admin token!");
    
    rotateSuccess.value = true;
    setTimeout(() => {
      closeRotateModal();
    }, 1500);

  } catch (err) {
    rotateError.value = String(err);
    addLog("error", `Failed to rotate password: ${err}`);
  } finally {
    rotating.value = false;
  }
}

onMounted(() => {
  loadConfig();

  // Listen to Tauri events
  listen<any>("propresenter:connected", (event) => {
    isProPresenterConnected.value = true;
    addLog("success", `Connected to ProPresenter: ${event.payload.host}:${event.payload.port}`);
  });

  listen<any>("propresenter:error", (event) => {
    isProPresenterConnected.value = false;
    addLog("error", `ProPresenter Connection Error: ${event.payload.error}`);
  });

  listen<any>("propresenter:disconnected", (_event) => {
    isProPresenterConnected.value = false;
    addLog("info", "Disconnected from ProPresenter");
  });

  listen<any>("propresenter:invalid-json", (event) => {
    addLog("warning", `Received invalid JSON chunk from ProPresenter: ${event.payload.error}`);
  });

  listen<SlideUpdate>("propresenter:slide-update", (event) => {
    const slide = event.payload;
    lastNotes.value = slide.notes;
    lastParsed.value = slide.parsed;
    lastMediaStarted.value = slide.media_started;

    let desc = `Slide update (gen ${slide.generation}): `;
    if (slide.parsed.regular_tags.length > 0) {
      desc += `Regular Tag: "${slide.parsed.regular_tags.join(", ")}"`;
    }
    if (slide.parsed.timed_tags.length > 0) {
      desc += `${slide.parsed.regular_tags.length > 0 ? " | " : ""}Video Timestamps: ${slide.parsed.timed_tags.map(t => `${t.raw_time} ${t.keyword}`).join(", ")}`;
    }
    if (slide.parsed.regular_tags.length === 0 && slide.parsed.timed_tags.length === 0) {
      desc += "No valid tags found in notes.";
    }
    addLog("event", desc);
  });

  listen<any>("propresenter:media-started-belated", (event) => {
    addLog("info", `[Belated Media Start] Slide gen ${event.payload.generation}: Video play trigger received belatedly.`);
  });

  listen<any>("redirect:regular-tag-applied", (event) => {
    addLog("success", `Regular tag redirect applied successfully`, event.payload.tag);
  });

  listen<any>("redirect:regular-tag-failed", (event) => {
    addLog("error", `Failed to apply regular tag redirect: ${event.payload.error}`, event.payload.tag);
  });

  listen<any>("redirect:timestamp-queued", (event) => {
    addLog("info", `Video timestamp redirect queued for t=${event.payload.seconds}s`, event.payload.tag);
  });

  listen<any>("redirect:timestamp-cancelled", (event) => {
    addLog("warning", `Video timestamp redirect cancelled: ${event.payload.reason}`, event.payload.tag);
  });

  listen<any>("redirect:write-started", (event) => {
    addLog("info", `KV redirect write started`, event.payload.tag);
  });

  listen<any>("redirect:write-succeeded", (event) => {
    addLog("success", `KV redirect write succeeded`, event.payload.tag);
  });

  listen<any>("redirect:write-failed", (event) => {
    addLog("error", `KV redirect write failed: ${event.payload.error}`, event.payload.tag);
  });

  listen<any>("worker:not-configured", (event) => {
    addLog("warning", `Worker operation ignored: ${event.payload.message}`);
  });
});
</script>

<template>
  <div class="dashboard">
    <!-- ONBOARDING FLOW -->
    <Onboarding v-if="!isOnboardingComplete" @complete="handleOnboardingComplete" />

    <!-- MAIN DASHBOARD -->
    <template v-else>
      <header class="app-header">
        <div class="header-logo">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="logo-icon"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15"/></svg>
          <h1>Tappy</h1>
          <span class="version">v0.1.0</span>
        </div>
        <div class="header-actions">
          <button class="btn btn-setup" @click="isOnboardingComplete = false">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
            Onboarding Setup
          </button>
          <div class="status-indicator">
            <span class="status-label">ProPresenter:</span>
            <span :class="['status-badge', isProPresenterConnected ? 'connected' : 'disconnected']">
              <span class="pulse-dot"></span>
              {{ isProPresenterConnected ? 'Connected' : 'Disconnected' }}
            </span>
          </div>
        </div>
      </header>

      <main class="grid-container">
        <!-- COLUMN 1: CONFIGURATION -->
        <section class="column">
          <!-- PROPRESENTER SETTINGS -->
          <ProPresenterCard
            v-model:host="propresenterHost"
            v-model:port="propresenterPort"
            :isConnected="isProPresenterConnected"
            @connect="connectToProPresenter"
            @disconnect="disconnectFromProPresenter"
          />

          <!-- CLOUDFLARE WORKER CONFIG -->
          <WorkerConfigCard
            v-model:baseUrl="workerBaseUrl"
            v-model:slug="workerSlug"
            v-model:minSpacingMs="minSpacingMs"
            @save="saveConfig"
            @open="openShortlink"
            @reset-password="showRotateModal = true"
          />

          <!-- TAG MAPPING EDITOR -->
          <TagMappingCard
            v-model:defaultUrl="defaultRedirectUrl"
            v-model:ownerId="linkOwnerId"
            v-model:tagMappings="tagMappings"
            :hasUnsavedChanges="hasUnsavedMappings"
            @push="pushTagMappings"
          />
        </section>

        <!-- COLUMN 2: REAL-TIME DISPLAY & FEED -->
        <section class="column">
          <!-- ACTIVE SLIDE CARD -->
          <ActiveSlideCard
            :last-notes="lastNotes"
            :last-parsed="lastParsed"
          />

          <!-- REAL-TIME LOG FEED -->
          <ActivityFeed
            :logs="logs"
            @clear="logs = []"
          />
        </section>
      </main>
    </template>

    <!-- ROTATE ADMIN PASSWORD MODAL -->
    <div v-if="showRotateModal" class="modal-overlay animate-fade">
      <div class="modal-card animate-slide">
        <div class="modal-header">
          <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="modal-icon"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
          <h2>Reset Tappy Password</h2>
        </div>
        <div class="modal-body">
          <p class="warning-text">
            <strong>Warning:</strong> This will rotate the <code>ADMIN_TOKEN</code> secret on your Cloudflare Worker. The worker will require this new token for all incoming commands. Your currently configured redirect tags will be immediately synchronized.
          </p>
          <p class="info-text">
            To redeploy and set the secret, you will need your <strong>Cloudflare Account ID</strong> and <strong>User API Token</strong>.
            These credentials are processed in-memory and are <strong>never saved to disk</strong>.
          </p>

          <div class="form-container">
            <div class="input-group">
              <label for="rotate-acc-id">Cloudflare Account ID</label>
              <input id="rotate-acc-id" v-model="rotateAccountId" placeholder="Enter your 32-character Account ID" :disabled="rotating" />
            </div>
            <div class="input-group">
              <label for="rotate-api-token">Cloudflare User API Token</label>
              <input id="rotate-api-token" type="password" v-model="rotateApiToken" placeholder="Enter your Cloudflare API Token" :disabled="rotating" />
            </div>
          </div>

          <!-- Status / Error Messages -->
          <div v-if="rotateError" class="status-msg error">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            <span>Reset failed: {{ rotateError }}</span>
          </div>

          <div v-if="rotateSuccess" class="status-msg success">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            <span>Password reset and tag synchronization complete!</span>
          </div>
        </div>

        <div class="modal-actions">
          <button class="btn btn-secondary" :disabled="rotating" @click="closeRotateModal">
            Cancel
          </button>
          <button 
            class="btn btn-danger" 
            :disabled="rotating || !rotateAccountId.trim() || !rotateApiToken.trim()" 
            @click="confirmRotatePassword"
          >
            {{ rotating ? 'Resetting & Syncing...' : 'Reset Password' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* CSS VARIABLES - Automatically aligned with system light/dark settings */
:root {
  /* Light mode palette */
  --bg-color: #f3f4f6;
  --bg-gradient: radial-gradient(at 0% 0%, rgba(203, 213, 225, 0.4) 0, transparent 50%),
                 radial-gradient(at 100% 100%, rgba(241, 245, 249, 0.4) 0, transparent 50%);
  --text-color: #0f172a;
  --text-muted: #475569;
  
  --card-bg: rgba(255, 255, 255, 0.65);
  --card-border: rgba(15, 23, 42, 0.08);
  --card-hover-border: rgba(15, 23, 42, 0.12);
  --card-shadow: 0 8px 32px rgba(15, 23, 42, 0.04);
  
  --input-bg: rgba(255, 255, 255, 0.85);
  --input-border: rgba(15, 23, 42, 0.12);
  --btn-secondary-bg: rgba(226, 232, 240, 0.8);
  --btn-secondary-color: #0f172a;
  --log-container-bg: rgba(241, 245, 249, 0.55);
  --notes-box-bg: rgba(255, 255, 255, 0.85);
  --border-color: rgba(15, 23, 42, 0.06);
}

@media (prefers-color-scheme: dark) {
  :root {
    /* Dark mode palette */
    --bg-color: #0b0f19;
    --bg-gradient: radial-gradient(at 0% 0%, rgba(20, 30, 80, 0.3) 0, transparent 50%),
                   radial-gradient(at 50% 0%, rgba(10, 80, 100, 0.2) 0, transparent 50%),
                   radial-gradient(at 100% 100%, rgba(29, 21, 60, 0.3) 0, transparent 50%);
    --text-color: #e2e8f0;
    --text-muted: #94a3b8;
    
    --card-bg: rgba(15, 23, 42, 0.45);
    --card-border: rgba(255, 255, 255, 0.06);
    --card-hover-border: rgba(255, 255, 255, 0.1);
    --card-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    
    --input-bg: rgba(30, 41, 59, 0.4);
    --input-border: rgba(255, 255, 255, 0.1);
    --btn-secondary-bg: rgba(51, 65, 85, 0.6);
    --btn-secondary-color: #f1f5f9;
    --log-container-bg: rgba(15, 23, 42, 0.4);
    --notes-box-bg: rgba(15, 23, 42, 0.6);
    --border-color: rgba(255, 255, 255, 0.06);
  }
}

/* Reset and Global Styles */
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  background-color: var(--bg-color);
  background-image: var(--bg-gradient);
  color: var(--text-color);
  font-family: 'Outfit', 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  min-height: 100vh;
  transition: background 0.3s ease, color 0.3s ease;
}

/* Dashboard Layout */
.dashboard {
  max-width: 1200px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  gap: 24px;
}

/* Header */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  background: var(--card-bg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid var(--card-border);
  border-radius: 16px;
  box-shadow: var(--card-shadow);
}

.header-logo {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo-icon {
  width: 28px;
  height: 28px;
  color: #38bdf8;
  filter: drop-shadow(0 0 8px rgba(56, 189, 248, 0.5));
}

.header-logo h1 {
  font-size: 24px;
  font-weight: 700;
  letter-spacing: -0.5px;
  background: linear-gradient(135deg, var(--text-color) 0%, var(--text-muted) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.header-logo .version {
  font-size: 11px;
  font-family: monospace;
  background: rgba(128, 128, 128, 0.15);
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--text-muted);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}

.btn-setup {
  background: rgba(56, 189, 248, 0.08);
  color: #38bdf8;
  border: 1px solid rgba(56, 189, 248, 0.15);
  border-radius: 8px;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.25s ease;
}

.btn-setup:hover {
  background: #38bdf8;
  color: #0f172a;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-label {
  font-size: 13px;
  color: var(--text-muted);
  font-weight: 500;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 9999px;
  font-size: 12px;
  font-weight: 600;
  transition: all 0.3s ease;
}

.status-badge.connected {
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.status-badge.disconnected {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.pulse-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: currentColor;
}

.status-badge.connected .pulse-dot {
  animation: pulse 1.8s infinite;
}

@keyframes pulse {
  0% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(52, 211, 153, 0.7);
  }
  70% {
    transform: scale(1);
    box-shadow: 0 0 0 6px rgba(52, 211, 153, 0);
  }
  100% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(52, 211, 153, 0);
  }
}

/* Grid Layout */
.grid-container {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 24px;
  flex-grow: 1;
}

@media (max-width: 900px) {
  .grid-container {
    grid-template-columns: 1fr;
  }
}

.column {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

/* Common Card & Form Styles */
.card {
  border-radius: 16px;
  display: flex;
  flex-direction: column;
  transition: transform 0.3s ease, box-shadow 0.3s ease, border 0.3s ease;
}

.card.glass {
  background: var(--card-bg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid var(--card-border);
  box-shadow: var(--card-shadow);
}

.card.glass:hover {
  border: 1px solid var(--card-hover-border);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.08);
}

.card.highlight-card {
  border: 1px solid rgba(56, 189, 248, 0.2);
}

.card.highlight-card:hover {
  border: 1px solid rgba(56, 189, 248, 0.4);
}

.card-header {
  padding: 18px 24px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 12px;
}

.card-icon {
  color: #38bdf8;
  opacity: 0.85;
}

.card-header h2 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-color);
}

.card-body {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.card-actions {
  margin-top: 10px;
}

input {
  width: 100%;
  background: var(--input-bg);
  border: 1px solid var(--input-border);
  border-radius: 10px;
  padding: 12px 16px;
  color: var(--text-color);
  font-size: 14px;
  font-family: inherit;
  transition: all 0.25s ease;
  outline: none;
}

input:focus {
  border-color: #38bdf8;
  box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.15);
}

/* Button UI */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 10px;
  padding: 12px 20px;
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  border: none;
  cursor: pointer;
  transition: all 0.25s ease;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.btn-primary {
  background: linear-gradient(135deg, #38bdf8 0%, #0284c7 100%);
  color: #ffffff;
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(56, 189, 248, 0.35);
}


.btn-inline {
  padding: 4px 10px !important;
  background: linear-gradient(135deg, #38bdf8 0%, #0284c7 100%);
  color: #ffffff;
}

.btn-inline:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(56, 189, 248, 0.35);
}

.btn-success {
  background: linear-gradient(135deg, #34d399 0%, #059669 100%);
  color: #ffffff;
}

.btn-success:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(52, 211, 153, 0.35);
}

.btn-secondary {
  background: var(--btn-secondary-bg);
  color: var(--btn-secondary-color);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover {
  background: rgba(128, 128, 128, 0.15);
  transform: translateY(-2px);
}

.btn:active {
  transform: translateY(0);
}

/* Modal Overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  padding: 20px;
}

/* Modal Card */
.modal-card {
  max-width: 550px;
  width: 100%;
  border-radius: 20px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.35);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(15, 23, 42, 0.1);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}

@media (prefers-color-scheme: dark) {
  .modal-card {
    background: rgba(20, 26, 42, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
}

.modal-header {
  padding: 24px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 12px;
}

.modal-icon {
  flex-shrink: 0;
}

.modal-header h2 {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-color);
}

.modal-body {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.warning-text {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--text-color);
  background: rgba(239, 68, 68, 0.08);
  border-left: 4px solid #ef4444;
  padding: 12px 16px;
  border-radius: 0 10px 10px 0;
}

.warning-text strong {
  color: #ef4444;
}

.warning-text code {
  background: rgba(239, 68, 68, 0.15);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: monospace;
}

.info-text {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--text-muted);
}

.modal-actions {
  padding: 20px 24px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  background: rgba(128, 128, 128, 0.02);
}

.btn-danger {
  background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
  color: #ffffff;
}

.btn-danger:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(239, 68, 68, 0.35);
}

.btn-danger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
</style>