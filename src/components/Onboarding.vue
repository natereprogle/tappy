<script setup lang="ts">

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

const emit = defineEmits<{
  (e: "complete", config: {
    baseUrl: string;
    adminToken: string;
    slug: string;
    defaultUrl: string;
    ownerId: string;
  }): void;
}>();

const currentStep = ref(1);

// Configuration state for Onboarding
const cloudflareAccountId = ref("");
const cloudflareApiToken = ref("");
const adminToken = ref(""); // Automatically generated in backend, stored here

interface ZoneInfo {
  id: string;
  name: string;
}

const zones = ref<ZoneInfo[]>([]);
const selectedZoneId = ref("");
const routingType = ref<"subdomain" | "path">("subdomain");
const subdomainValue = ref("tappy");
const pathValue = ref("redirect");

const fetchingZones = ref(false);
const fetchZonesError = ref("");
const domainsFetched = ref(false);

const baseUrl = ref(""); // Retrieved automatically after deployment
const slug = ref("");
const defaultUrl = ref("https://");
const ownerId = ref("tappy_user");

// Status validation states
const deployingWorker = ref(false);
const deployStatus = ref<"idle" | "success" | "error">("idle");
const deployErrorMessage = ref("");

const checkingSlug = ref(false);
const slugStatus = ref<"idle" | "available" | "taken" | "reusing" | "error">("idle");
const slugErrorMessage = ref("");
const initializingWorker = ref(false);

async function handleFetchZones() {
  if (!cloudflareApiToken.value.trim()) {
    fetchZonesError.value = "Please enter your User API Token first.";
    return;
  }

  fetchingZones.value = true;
  fetchZonesError.value = "";
  zones.value = [];

  try {
    const list = await invoke<ZoneInfo[]>("list_cloudflare_zones", {
      apiToken: cloudflareApiToken.value.trim(),
    });
    zones.value = list;
    if (list.length > 0) {
      selectedZoneId.value = list[0].id;
    }
    domainsFetched.value = true;
  } catch (err) {
    fetchZonesError.value = String(err);
    domainsFetched.value = false;
  } finally {
    fetchingZones.value = false;
  }
}

async function handleDeployWorker() {
  if (!cloudflareAccountId.value.trim()) {
    deployStatus.value = "error";
    deployErrorMessage.value = "Please enter your Cloudflare Account ID.";
    return;
  }
  if (!cloudflareApiToken.value.trim()) {
    deployStatus.value = "error";
    deployErrorMessage.value = "Please enter your Cloudflare User API Token.";
    return;
  }
  
  const selectedZone = zones.value.find(z => z.id === selectedZoneId.value);
  if (!selectedZone) {
    deployStatus.value = "error";
    deployErrorMessage.value = "Please fetch and select a Cloudflare Domain (Zone).";
    return;
  }

  deployingWorker.value = true;
  deployStatus.value = "idle";
  deployErrorMessage.value = "";

  try {
    const result = await invoke<{ base_url: string; admin_token: string }>("deploy_cloudflare_worker", {
      accountId: cloudflareAccountId.value.trim(),
      apiToken: cloudflareApiToken.value.trim(),
      zoneId: selectedZone.id,
      zoneName: selectedZone.name,
      subdomain: routingType.value === "subdomain" ? subdomainValue.value.trim() : null,
      path: routingType.value === "path" ? pathValue.value.trim() : null,
    });

    baseUrl.value = result.base_url;
    adminToken.value = result.admin_token;
    deployStatus.value = "success";
    // Wait a brief second to let user see success, then transition to Step 3
    setTimeout(() => {
      currentStep.value = 3;
    }, 800);
  } catch (err) {
    deployStatus.value = "error";
    deployErrorMessage.value = String(err);
  } finally {
    deployingWorker.value = false;
  }
}

async function verifySlugAvailability() {
  if (!baseUrl.value) {
    slugStatus.value = "error";
    slugErrorMessage.value = "Please deploy your worker in Step 2 first.";
    return;
  }
  if (!slug.value.trim()) {
    slugStatus.value = "error";
    slugErrorMessage.value = "Please enter a Redirect Slug.";
    return;
  }

  checkingSlug.value = true;
  slugStatus.value = "idle";
  slugErrorMessage.value = "";

  try {
    const config = await invoke<{ defaultUrl: string; ownerId: string } | null>("download_slug_config", {
      baseUrl: baseUrl.value,
      adminToken: adminToken.value,
      slug: slug.value.trim(),
    });

    if (config) {
      if (config.defaultUrl) {
        defaultUrl.value = config.defaultUrl;
        ownerId.value = config.ownerId || "tappy_user";
        slugStatus.value = "reusing";
      } else {
        slugStatus.value = "taken";
      }
    } else {
      slugStatus.value = "available";
    }
  } catch (err) {
    slugStatus.value = "error";
    slugErrorMessage.value = String(err);
  } finally {
    checkingSlug.value = false;
  }
}

async function initializeAndFinish() {
  if (slugStatus.value !== "available" && slugStatus.value !== "reusing") return;
  
  if (!defaultUrl.value.startsWith("http://") && !defaultUrl.value.startsWith("https://")) {
    slugStatus.value = "error";
    slugErrorMessage.value = "Fallback URL must start with http:// or https://";
    return;
  }

  initializingWorker.value = true;
  slugErrorMessage.value = "";

  try {
    // 1. Configure the worker settings in AppState
    await invoke("configure_worker", {
      baseUrl: baseUrl.value,
      adminToken: adminToken.value,
      slug: slug.value.trim(),
    });

    // 2. Push the master link setup to the Cloudflare KV database (which verifies the endpoint is created)
    await invoke("create_or_update_master_link", {
      defaultUrl: defaultUrl.value,
      ownerId: ownerId.value,
      tags: {},
    });

    // 3. Complete onboarding
    emit("complete", {
      baseUrl: baseUrl.value,
      adminToken: adminToken.value,
      slug: slug.value.trim(),
      defaultUrl: defaultUrl.value,
      ownerId: ownerId.value,
    });
  } catch (err) {
    slugStatus.value = "error";
    slugErrorMessage.value = `Worker initialization failed: ${err}`;
  } finally {
    initializingWorker.value = false;
  }
}

async function openLink() {
  await openUrl('https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=%5B%0A%20%20%7B%0A%20%20%20%20%22key%22%3A%20%22workers_scripts%22%2C%0A%20%20%20%20%22type%22%3A%20%22edit%22%0A%20%20%7D%2C%0A%20%20%7B%0A%20%20%20%20%22key%22%3A%20%22workers_kv_storage%22%2C%0A%20%20%20%20%22type%22%3A%20%22edit%22%0A%20%20%7D%2C%0A%20%20%7B%0A%20%20%20%20%22key%22%3A%20%22workers_routes%22%2C%0A%20%20%20%20%22type%22%3A%20%22edit%22%0A%20%20%7D%2C%0A%20%20%7B%0A%20%20%20%20%22key%22%3A%20%22dns%22%2C%0A%20%20%20%20%22type%22%3A%20%22edit%22%0A%20%20%7D%2C%0A%20%20%7B%0A%20%20%20%20%22key%22%3A%20%22zone%22%2C%0A%20%20%20%20%22type%22%3A%20%22read%22%0A%20%20%7D%0A%5D&accountId=*&zoneId=all&name=Tappy%20Token');
}
</script>

<template>
  <div class="onboarding-container glass">
    <div class="onboarding-card">
      <!-- Title & Wizard Progress -->
      <div class="onboarding-header">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="logo-icon"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15"/></svg>
        <h1>Welcome to Tappy! Let's get set up.</h1>
        <p class="subtitle">Complete these 3 steps to automatically deploy and configure your redirection backend.</p>
      </div>

      <!-- Step Indicators -->
      <div class="step-indicators">
        <div :class="['step-tab', currentStep >= 1 ? 'active' : '']">
          <span class="step-num">1</span>
          <span class="step-title">Credentials</span>
        </div>
        <div class="step-divider"></div>
        <div :class="['step-tab', currentStep >= 2 ? 'active' : '']">
          <span class="step-num">2</span>
          <span class="step-title">Deploy Backend</span>
        </div>
        <div class="step-divider"></div>
        <div :class="['step-tab', currentStep >= 3 ? 'active' : '']">
          <span class="step-num">3</span>
          <span class="step-title">Set up Slug</span>
        </div>
      </div>

      <!-- STEP 1 content -->
      <div v-if="currentStep === 1" class="step-content animate-fade">
        <h3>Step 1: Get Cloudflare Credentials</h3>
        <p class="instructions-text">
          Tappy will automatically create your Workers KV Database, build the redirection script, bind it, and deploy it to your Cloudflare Account. We need two credentials, which you can get by walking through the steps below:
        </p>
        <ol class="step-list">
          <li><strong>Obtain your Cloudflare Account ID</strong>
            <br />
            On the left panel of the Cloudflare Dashboard, select <strong>Compute &rarr; Workers & Pages</strong>. On the right sidebar, under Account Details, copy the <strong>Account ID</strong>.
          </li>
          <li><strong>Create a Cloudflare User API Token</strong>
            <br />
            Click <button class="btn btn-inline" @click="openLink">this button</button> to create a new Cloudflare User API Token. Under <strong>Account Resources</strong>, select your account. Under <strong>Zone Resources</strong>, select <strong>Specific zone</strong>, then select your domain you want to use with Tappy. Select <strong>Continue to summary</strong> and finally <strong>Create Token</strong>. Write this down, you cannot retrieve it again after it's lost!
          </li>
        </ol>
        <div class="step-actions">
          <div></div>
          <button class="btn btn-primary" @click="currentStep = 2">
            Next: Enter Credentials
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
          </button>
        </div>
      </div>

      <!-- STEP 2 content -->
      <div v-if="currentStep === 2" class="step-content animate-fade">
        <h3>Step 2: Deploy Redirection Script</h3>
        <p class="instructions-text font-small">
          Enter your Cloudflare details, fetch your domains, and select how you want to route your worker. Tappy will automatically configure DNS and generate a secure admin token behind the scenes.
        </p>

        <div class="form-container">
          <div class="input-group">
            <label for="ob-acc-id">Cloudflare Account ID</label>
            <input id="ob-acc-id" v-model="cloudflareAccountId" @input="domainsFetched = false" placeholder="Enter your 32-character Account ID" />
          </div>
          <div class="input-group">
            <label for="ob-api-tok">Cloudflare User API Token</label>
            <input id="ob-api-tok" type="password" v-model="cloudflareApiToken" @input="domainsFetched = false" placeholder="Enter your Cloudflare API Token" />
          </div>
          
          <div class="zone-fetch-actions">
            <button class="btn btn-secondary btn-small" :disabled="fetchingZones || !cloudflareApiToken" @click="handleFetchZones">
              {{ fetchingZones ? 'Fetching domains...' : 'Fetch Domains (Zones)' }}
            </button>
            <span v-if="fetchZonesError" class="fetch-error">{{ fetchZonesError }}</span>
          </div>

          <div v-if="zones.length > 0" class="zone-setup-group animate-slide">
            <div class="input-group">
              <label for="ob-zone">Target Domain (Zone)</label>
              <select id="ob-zone" v-model="selectedZoneId" class="select-input">
                <option v-for="z in zones" :key="z.id" :value="z.id">{{ z.name }}</option>
              </select>
            </div>

            <div class="input-group">
              <label>Routing Method</label>
              <div class="radio-group">
                <label class="radio-label">
                  <input type="radio" v-model="routingType" value="subdomain" />
                  Subdomain (e.g., tappy.yourdomain.com)
                </label>
                <label class="radio-label">
                  <input type="radio" v-model="routingType" value="path" />
                  Path route (e.g., yourdomain.com/redirect)
                </label>
              </div>
            </div>

            <div class="input-group">
              <label v-if="routingType === 'subdomain'" for="ob-subdomain-val">Subdomain Prefix</label>
              <label v-else for="ob-path-val">Path Prefix</label>
              <input v-if="routingType === 'subdomain'" id="ob-subdomain-val" v-model="subdomainValue" placeholder="tappy" />
              <input v-else id="ob-path-val" v-model="pathValue" placeholder="redirect" />
            </div>
          </div>

          <!-- Status Messages -->
          <div v-if="deployStatus === 'success'" class="status-msg success">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            <span>Worker deployed successfully! Custom URL: {{ baseUrl }}</span>
          </div>
          <div v-if="deployStatus === 'error'" class="status-msg error">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            <span>Deployment failed: {{ deployErrorMessage }}</span>
          </div>
        </div>

        <div class="step-actions">
          <button class="btn btn-secondary" :disabled="deployingWorker" @click="currentStep = 1">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
            Back
          </button>
          <button 
            class="btn btn-success" 
            :disabled="deployingWorker || !cloudflareAccountId || !cloudflareApiToken || !domainsFetched || zones.length === 0 || !selectedZoneId" 
            @click="handleDeployWorker"
          >
            {{ deployingWorker ? 'Deploying and Routing...' : 'Deploy & Route Worker' }}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
          </button>
        </div>
      </div>

      <!-- STEP 3 content -->
      <div v-if="currentStep === 3" class="step-content animate-fade">
        <h3>Step 3: Setup Redirection Endpoint & Verify Availability</h3>
        <p class="instructions-text font-small">
          Choose the redirect endpoint slug (DNS path) that users will visit. Tappy will query your deployed worker to ensure this slug is not already configured before initializing.
        </p>

        <div class="form-container">
          <div class="endpoint-setup">
            <div class="input-row align-end">
              <div class="input-group flex-grow">
                <label for="ob-slug">Endpoint Name (Slug)</label>
                <div class="slug-input-wrapper">
                  <span class="slug-domain">{{ baseUrl }}/</span>
                  <input id="ob-slug" v-model="slug" placeholder="e.g. sermon" @keyup.enter="verifySlugAvailability" />
                </div>
              </div>
              <button 
                class="btn btn-check" 
                :disabled="checkingSlug || !slug.trim()" 
                @click="verifySlugAvailability"
              >
                {{ checkingSlug ? 'Checking...' : 'Check Availability' }}
              </button>
            </div>

            <!-- Verification Messages -->
            <div v-if="slugStatus === 'available'" class="status-msg success">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
              <span>Endpoint is available! Setup the fallback parameters below.</span>
            </div>
            <div v-if="slugStatus === 'reusing'" class="status-msg warning">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
              <span>Endpoint already exists, reusing existing configuration</span>
            </div>
            <div v-if="slugStatus === 'taken'" class="status-msg error">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              <span>This endpoint cannot be used. Please choose another slug.</span>
            </div>
            <div v-if="slugStatus === 'error'" class="status-msg error">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              <span>Error checking endpoint: {{ slugErrorMessage }}</span>
            </div>
          </div>

          <!-- Final Fallback setup, only visible when slug is verified available/reusing -->
          <div v-if="slugStatus === 'available' || slugStatus === 'reusing'" class="fallback-setup animate-slide">
            <div class="input-row">
              <div class="input-group flex-grow">
                <label for="ob-fallback">Default Fallback URL (No active tags)</label>
                <input id="ob-fallback" v-model="defaultUrl" placeholder="https://example.com" />
              </div>
              <div class="input-group">
                <label for="ob-owner">Owner ID</label>
                <input id="ob-owner" v-model="ownerId" placeholder="tappy_user" />
              </div>
            </div>
          </div>
        </div>

        <div class="step-actions">
          <button class="btn btn-secondary" :disabled="initializingWorker" @click="currentStep = 2">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
            Back
          </button>
          <button 
            class="btn btn-success" 
            :disabled="(slugStatus !== 'available' && slugStatus !== 'reusing') || initializingWorker || !defaultUrl.startsWith('http')" 
            @click="initializeAndFinish"
          >
            {{ initializingWorker ? 'Initializing...' : 'Initialize & Finish' }}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.onboarding-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 80vh;
  width: 100%;
}

.onboarding-card {
  max-width: 650px;
  width: 100%;
  padding: 36px;
  display: flex;
  flex-direction: column;
  gap: 28px;
}

.onboarding-header {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.logo-icon {
  width: 48px;
  height: 48px;
  color: #38bdf8;
  filter: drop-shadow(0 0 10px rgba(56, 189, 248, 0.4));
  margin-bottom: 4px;
}

.onboarding-header h1 {
  font-size: 26px;
  font-weight: 800;
  letter-spacing: -0.5px;
  background: linear-gradient(135deg, var(--text-color) 0%, var(--text-muted) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.subtitle {
  color: var(--text-muted);
  font-size: 14px;
  max-width: 480px;
  line-height: 1.5;
}

/* Step Progress Indicator */
.step-indicators {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: rgba(128, 128, 128, 0.08);
  border: 1px solid var(--border-color);
  padding: 8px 16px;
  border-radius: 30px;
}

.step-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  opacity: 0.4;
  transition: all 0.3s ease;
}

.step-tab.active {
  opacity: 1;
}

.step-num {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--text-muted);
  color: var(--bg-color);
  font-size: 12px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}

.step-tab.active .step-num {
  background: #38bdf8;
  color: #0b0f19;
  box-shadow: 0 0 10px rgba(56, 189, 248, 0.4);
}

.step-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-color);
}

.step-divider {
  width: 32px;
  height: 1px;
  background: var(--border-color);
}

/* Step Content Styles */
.step-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.step-content h3 {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-color);
}

.instructions-text {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-muted);
}

.font-small {
  font-size: 13.5px;
}

.color-muted {
  opacity: 0.8;
}

.step-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-left: 20px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-color);
}

.step-list li {
  margin-bottom: 4px;
}

.step-list li strong {
  color: #38bdf8;
  font-weight: 600;
}

.form-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: rgba(128, 128, 128, 0.03);
  padding: 20px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
}

.endpoint-setup {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.slug-input-wrapper {
  display: flex;
  align-items: center;
  background: var(--input-bg);
  border: 1px solid var(--input-border);
  border-radius: 10px;
  padding: 0 16px;
  overflow: hidden;
}

.slug-input-wrapper input {
  border: none;
  background: none;
  padding: 12px 0;
  border-radius: 0;
}

.slug-input-wrapper input:focus {
  box-shadow: none;
  background: none;
}

.slug-domain {
  font-size: 13px;
  color: var(--text-muted);
  font-family: monospace;
  white-space: nowrap;
  user-select: none;
  opacity: 0.65;
  padding-right: 4px;
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn-check {
  background: rgba(56, 189, 248, 0.1);
  color: #38bdf8;
  border: 1px solid rgba(56, 189, 248, 0.2);
  height: 44px;
  font-weight: 600;
  border-radius: 10px;
  cursor: pointer;
  padding: 0 16px;
  transition: all 0.25s ease;
}

.btn-check:hover:not(:disabled) {
  background: #38bdf8;
  color: #0f172a;
}

.btn-check:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-msg {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
  padding: 10px 14px;
  border-radius: 8px;
  margin-top: 4px;
}

.status-msg.success {
  background: rgba(16, 185, 129, 0.1);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.2);
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

.fallback-setup {
  display: flex;
  flex-direction: column;
  gap: 16px;
  border-top: 1px solid var(--border-color);
  padding-top: 16px;
}

.step-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 12px;
}

/* Animations */
.animate-fade {
  animation: fadeIn 0.4s ease-out;
}

.animate-slide {
  animation: slideDown 0.3s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes slideDown {
  from { opacity: 0; transform: translateY(-8px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Inputs and Groups */
.input-row {
  display: flex;
  gap: 16px;
}

.input-row.align-end {
  align-items: flex-end;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
}

.input-group label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.select-input {
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
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23475569' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
  background-repeat: no-repeat;
  background-position: right 16px center;
  background-size: 16px;
}

.select-input:focus {
  border-color: #38bdf8;
  box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.15);
}

.zone-fetch-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 4px 0;
}

.btn-small {
  padding: 8px 16px;
  font-size: 12.5px;
  border-radius: 8px;
}

.fetch-error {
  color: #f87171;
  font-size: 13px;
  font-weight: 500;
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 4px 0;
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13.5px;
  color: var(--text-color);
  cursor: pointer;
  user-select: none;
}

.radio-label input[type="radio"] {
  width: auto;
  margin: 0;
  cursor: pointer;
}

.zone-setup-group {
  display: flex;
  flex-direction: column;
  gap: 16px;
  border-top: 1px solid var(--border-color);
  padding-top: 16px;
  margin-top: 4px;
}
</style>
