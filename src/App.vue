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
const activeTab = ref<"dashboard" | "settings" | "advanced">("dashboard");

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

    // load_settings already restores the admin token from the OS keychain
    // into AppState. We call get_worker_config here only to sync the token
    // back to the Vue layer so that subsequent saveConfig() calls include it.
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
    <Onboarding
      v-if="!isOnboardingComplete"
      :can-cancel="!!workerBaseUrl"
      @complete="handleOnboardingComplete"
      @close="isOnboardingComplete = true"
    />

    <!-- MAIN DASHBOARD -->
    <template v-else>
      <header class="app-header">
        <div class="header-top">
          <div class="header-logo">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="logo-icon"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15"/></svg>
            <h1>Tappy</h1>
            <span class="version">v0.1.0</span>
          </div>
          
          <div class="header-right">
            <div class="status-dot-wrapper" :title="'ProPresenter: ' + (isProPresenterConnected ? 'Connected' : 'Disconnected')">
              <span :class="['status-dot', isProPresenterConnected ? 'connected' : 'disconnected']"></span>
            </div>

            <button 
              :class="['btn-header-connect', isProPresenterConnected ? 'connected' : '']"
              :title="isProPresenterConnected ? 'Disconnect from ProPresenter' : 'Connect to ProPresenter'"
              @click="isProPresenterConnected ? disconnectFromProPresenter() : connectToProPresenter()"
            >
              <svg v-if="!isProPresenterConnected" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="9" y1="9" x2="15" y2="15"/><line x1="15" y1="9" x2="9" y2="15"/></svg>
              <span>{{ isProPresenterConnected ? 'Disconnect' : 'Connect' }}</span>
            </button>
          </div>
        </div>
        
        <div class="tab-switcher">
          <button :class="['tab-btn', activeTab === 'dashboard' ? 'active' : '']" @click="activeTab = 'dashboard'">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></svg>
            Dashboard
          </button>
          <button :class="['tab-btn', activeTab === 'settings' ? 'active' : '']" @click="activeTab = 'settings'">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
            Settings
          </button>
          <button :class="['tab-btn', activeTab === 'advanced' ? 'active' : '']" @click="activeTab = 'advanced'">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m16.07 16.07 1.41 1.41"/><path d="m16.07 7.93 1.41-1.41"/><path d="m7.93 16.07-1.41 1.41"/><path d="m7.93 7.93-1.41-1.41"/><path d="M12 12V2"/><path d="M12 12v10"/><path d="M12 12H2"/><path d="M12 12h10"/></svg>
            Advanced
          </button>
        </div>
      </header>

      <main class="app-content">
        <!-- TAB 1: SHOW DASHBOARD -->
        <div v-if="activeTab === 'dashboard'" class="tab-panel animate-fade">
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
        </div>

        <!-- TAB 2: CONFIGURATION -->
        <div v-else-if="activeTab === 'settings'" class="tab-panel animate-fade">
          <!-- PROPRESENTER SETTINGS -->
          <ProPresenterCard
            v-model:host="propresenterHost"
            v-model:port="propresenterPort"
            :isConnected="isProPresenterConnected"
            @connect="connectToProPresenter"
            @disconnect="disconnectFromProPresenter"
          />

          <!-- TAG MAPPING EDITOR -->
          <TagMappingCard
            v-model:defaultUrl="defaultRedirectUrl"
            v-model:ownerId="linkOwnerId"
            v-model:tagMappings="tagMappings"
            :hasUnsavedChanges="hasUnsavedMappings"
            @push="pushTagMappings"
          />
        </div>

        <!-- TAB 3: ADVANCED -->
        <div v-else-if="activeTab === 'advanced'" class="tab-panel animate-fade">
          <!-- CLOUDFLARE WORKER CONFIG -->
          <WorkerConfigCard
            v-model:baseUrl="workerBaseUrl"
            v-model:slug="workerSlug"
            v-model:minSpacingMs="minSpacingMs"
            @save="saveConfig"
            @open="openShortlink"
            @reset-password="showRotateModal = true"
          />

          <!-- REDO ONBOARDING -->
          <div class="card redo-onboarding-card">
            <div class="card-header">
              <div class="header-icon">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
              </div>
              <h3>System Setup</h3>
            </div>
            <p class="card-description">Need to reconfigure your environment? You can rerun the initial setup process to rebuild your Cloudflare Worker or update your credentials.</p>
            <button class="btn-redo-onboarding" @click="isOnboardingComplete = false">
              Redo Onboarding Setup
            </button>
          </div>
        </div>
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
  --primary-color: #6366f1;
  --primary-hover: #4f46e5;
  --bg-color: #f8fafc;
  --text-color: #0f172a;
  --text-muted: #64748b;
  
  --card-bg: #ffffff;
  --card-border: #e2e8f0;
  --card-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
  --card-radius: 16px;
  --card-hover-border: var(--primary-color);
  
  --input-bg: #ffffff;
  --input-border: #e2e8f0;
  --btn-secondary-bg: #f1f5f9;
  --btn-secondary-color: #475569;
  --log-container-bg: #f8fafc;
  --notes-box-bg: #f8fafc;
  --border-color: #e2e8f0;

  --success: #10b981;
  --warning: #f59e0b;
  --error: #ef4444;
  --info: #3b82f6;
}

@media (prefers-color-scheme: dark) {
  :root {
    --primary-color: #818cf8;
    --primary-hover: #6366f1;
    --bg-color: #020617;
    --text-color: #f1f5f9;
    --text-muted: #94a3b8;
    
    --card-bg: #1e293b;
    --card-border: rgba(255, 255, 255, 0.05);
    --card-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -2px rgba(0, 0, 0, 0.2);
    
    --input-bg: #0f172a;
    --input-border: #334155;
    --btn-secondary-bg: #334155;
    --btn-secondary-color: #e2e8f0;
    --log-container-bg: #0f172a;
    --notes-box-bg: #0f172a;
    --border-color: #334155;
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
  color: var(--text-color);
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  transition: background 0.3s ease, color 0.3s ease;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

/* Dashboard Layout */
.dashboard {
  max-width: 520px;
  width: 100%;
  margin: 0 auto;
  padding: 24px 16px;
  display: flex;
  flex-direction: column;
  height: 100vh;
  gap: 20px;
  box-sizing: border-box;
}

/* Compact Header Styling */
.app-header {
  display: flex;
  flex-direction: column;
  padding: 16px;
  background: var(--card-bg);
  border: 1px solid var(--card-border);
  border-radius: 20px;
  box-shadow: var(--card-shadow);
  gap: 16px;
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.header-logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo-icon {
  width: 24px;
  height: 24px;
  color: var(--primary-color);
  filter: drop-shadow(0 0 8px rgba(99, 102, 241, 0.3));
}

.header-logo h1 {
  font-size: 20px;
  font-weight: 800;
  color: var(--text-color);
  letter-spacing: -0.5px;
}

.header-logo .version {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  background: var(--btn-secondary-bg);
  padding: 2px 6px;
  border-radius: 6px;
  color: var(--text-muted);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

/* Status Indicator Dot */
.status-dot-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  transition: all 0.3s ease;
}

.status-dot.connected {
  background-color: var(--success);
  box-shadow: 0 0 12px var(--success);
}

.status-dot.disconnected {
  background-color: var(--error);
  box-shadow: 0 0 12px var(--error);
}

/* Onboarding Setup Button */
.btn-onboarding-setup {
  background: var(--btn-secondary-bg);
  border: none;
  color: var(--text-muted);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-onboarding-setup:hover {
  color: var(--text-color);
  background: var(--border-color);
}

/* Header Connection Button */
.btn-header-connect {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--primary-color);
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-header-connect:hover {
  background: var(--primary-hover);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

.btn-header-connect.connected {
  background: var(--btn-secondary-bg);
  color: var(--text-color);
}

.btn-header-connect.connected:hover {
  background: var(--error);
  color: white;
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
}

/* Tab Switcher */
.tab-switcher {
  display: flex;
  background: var(--btn-secondary-bg);
  padding: 4px;
  border-radius: 12px;
  gap: 4px;
}

.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  border-radius: 8px;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.tab-btn:hover {
  color: var(--text-color);
}

.tab-btn.active {
  background: var(--card-bg);
  color: var(--primary-color);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

/* Scrollable Container Content */
.app-content {
  flex-grow: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 24px;
}

/* Custom Scrollbar for App Content */
.app-content::-webkit-scrollbar {
  width: 6px;
}

.app-content::-webkit-scrollbar-track {
  background: transparent;
}

.app-content::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.25);
  border-radius: 3px;
}

.app-content::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.4);
}

/* Tab Panel Grid & Animation */
.tab-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
}

/* Common Card Styles - Dense and Engineered */
.card {
  border-radius: 12px;
  background: var(--card-bg);
  border: 1px solid var(--card-border);
  box-shadow: var(--card-shadow);
  display: flex;
  flex-direction: column;
  transition: border-color 0.2s ease;
}

.card:hover {
  border-color: var(--card-hover-border);
}

.card-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 10px;
}

.card-icon {
  color: var(--text-muted);
  opacity: 0.85;
}

.card-header h2 {
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-color);
}

.card-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.card-actions {
  margin-top: 6px;
}

/* Inputs and Forms */
input {
  width: 100%;
  background: var(--input-bg);
  border: 1px solid var(--input-border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-color);
  font-size: 13px;
  transition: all 0.2s ease;
  outline: none;
}

input:focus {
  border-color: var(--card-hover-border);
  box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.15);
}

/* Button UI - Sharp & Solid */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: 6px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-primary {
  background: #0969da;
  color: #ffffff;
}

.btn-primary:hover {
  background: #0c75ef;
}

.btn-success {
  background: #2ea44f;
  color: #ffffff;
}

.btn-success:hover {
  background: #2c974b;
}

.btn-secondary {
  background: var(--btn-secondary-bg);
  color: var(--btn-secondary-color);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover {
  background: rgba(128, 128, 128, 0.1);
}

.btn:active {
  transform: scale(0.98);
}

/* Animations */
.animate-fade {
  animation: fadeIn 0.25s ease-out;
}

.animate-slide {
  animation: slideDown 0.2s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes slideDown {
  from { opacity: 0; transform: translateY(-8px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Modal Overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  padding: 16px;
}

/* Modal Card */
.modal-card {
  max-width: 440px;
  width: 100%;
  border-radius: 12px;
  box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid rgba(15, 23, 42, 0.1);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}

@media (prefers-color-scheme: dark) {
  .modal-card {
    background: rgba(20, 26, 42, 0.98);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
}

.modal-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 10px;
}

.modal-header h2 {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-color);
}

.modal-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.warning-text {
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--text-color);
  background: rgba(239, 68, 68, 0.08);
  border-left: 3px solid #ef4444;
  padding: 10px 12px;
  border-radius: 0 6px 6px 0;
}

.warning-text strong {
  color: #ef4444;
}

.warning-text code {
  background: rgba(239, 68, 68, 0.15);
  padding: 2px 4px;
  border-radius: 3px;
  font-family: monospace;
}

.info-text {
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--text-muted);
}

.modal-actions {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  background: rgba(128, 128, 128, 0.02);
}

.btn-danger {
  background: #ef4444;
  color: #ffffff;
}

.btn-danger:hover:not(:disabled) {
  background: #dc2626;
}

.btn-danger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-msg {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  padding: 8px 10px;
  border-radius: 6px;
}

.status-msg.success {
  background: rgba(46, 164, 79, 0.1);
  color: #2ea44f;
  border: 1px solid rgba(46, 164, 79, 0.2);
}

.status-msg.error {
  background: rgba(239, 68, 68, 0.1);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.2);
}

.status-msg.warning {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
  border: 1px solid rgba(245, 158, 11, 0.2);
}

.form-container {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: rgba(128, 128, 128, 0.03);
  padding: 12px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.input-group label {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* Redo Onboarding Card */
.redo-onboarding-card {
  padding: 20px;
  gap: 16px;
  text-align: center;
  background: linear-gradient(to bottom right, var(--card-bg), var(--bg-color));
}

.redo-onboarding-card .card-header {
  border-bottom: none;
  padding: 0;
  justify-content: center;
  flex-direction: column;
}

.redo-onboarding-card .header-icon {
  width: 48px;
  height: 48px;
  background: var(--btn-secondary-bg);
  color: var(--primary-color);
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 4px;
}

.redo-onboarding-card h3 {
  font-size: 18px;
  font-weight: 800;
  letter-spacing: -0.5px;
  margin: 0;
}

.redo-onboarding-card .card-description {
  font-size: 14px;
  color: var(--text-muted);
  line-height: 1.6;
  margin: 0;
}

.btn-redo-onboarding {
  background: var(--btn-secondary-bg);
  color: var(--text-color);
  border: 1px solid var(--border-color);
  padding: 10px 20px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-redo-onboarding:hover {
  background: var(--border-color);
  transform: translateY(-1px);
}
</style>