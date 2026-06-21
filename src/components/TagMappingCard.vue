<script setup lang="ts">
import {ref} from "vue";

const props = defineProps<{
  defaultUrl: string;
  ownerId: string;
  tagMappings: Record<string, string>;
  hasUnsavedChanges?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:defaultUrl", val: string): void;
  (e: "update:ownerId", val: string): void;
  (e: "update:tagMappings", val: Record<string, string>): void;
  (e: "push"): void;
}>();

const newTag = ref("");
const newUrl = ref("");
const addError = ref("");
const editingTag = ref("");

function saveMapping() {
  addError.value = "";
  const tag = newTag.value.trim().toLowerCase();
  const url = newUrl.value.trim();

  if (!tag) {
    addError.value = "Tag cannot be empty.";
    return;
  }
  if (!url) {
    addError.value = "Redirect URL cannot be empty.";
    return;
  }
  if (!url.startsWith("http://") && !url.startsWith("https://")) {
    addError.value = "Redirect URL must start with http:// or https://.";
    return;
  }

  const updated = {...props.tagMappings};

  if (editingTag.value) {
    // If we changed the tag name, make sure the new name isn't already taken by another mapping
    if (tag !== editingTag.value && updated[tag]) {
      addError.value = `Mapping for tag '#${tag}' already exists.`;
      return;
    }
    // Delete the old key if it was renamed
    if (tag !== editingTag.value) {
      delete updated[editingTag.value];
    }
  } else {
    // Adding a new mapping
    if (updated[tag]) {
      addError.value = `Mapping for tag '#${tag}' already exists.`;
      return;
    }
  }

  updated[tag] = url;
  emit("update:tagMappings", updated);

  cancelEdit();
}

function startEdit(tag: string, url: string) {
  addError.value = "";
  editingTag.value = tag;
  newTag.value = tag;
  newUrl.value = url;
}

function cancelEdit() {
  editingTag.value = "";
  newTag.value = "";
  newUrl.value = "";
  addError.value = "";
}

function removeMapping(tag: string) {
  if (editingTag.value === tag) {
    cancelEdit();
  }
  const updated = {...props.tagMappings};
  delete updated[tag];
  emit("update:tagMappings", updated);
}
</script>

<template>
  <div class="card glass">
    <div class="card-header">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="card-icon">
        <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/>
        <line x1="7" y1="7" x2="7.01" y2="7"/>
      </svg>
      <h2>Redirect Tag Mapping</h2>
    </div>
    <div class="card-body">
      <div class="input-row">
        <div class="input-group">
          <label for="fallback-url">Fallback URL</label>
          <input
              id="fallback-url"
              :value="defaultUrl"
              @input="emit('update:defaultUrl', ($event.target as HTMLInputElement).value)"
              placeholder="e.g. https://mychurch.com"
          />
        </div>
        <div class="input-group">
          <label for="owner-id">Owner ID</label>
          <input
              id="owner-id"
              :value="ownerId"
              @input="emit('update:ownerId', ($event.target as HTMLInputElement).value)"
              placeholder="e.g. church_staff"
          />
        </div>
      </div>

      <div class="mappings-editor">
        <span class="editor-label">Mapped Redirect Tags</span>

        <div class="mappings-list">
          <div v-for="(url, tag) in tagMappings" :key="tag" class="mapping-item">
            <span class="mapping-tag">#{{ tag }}</span>
            <span class="mapping-arrow">→</span>
            <span class="mapping-url" :title="url">{{ url }}</span>
            <div class="action-buttons">
              <button class="btn-edit" @click="startEdit(tag as string, url as string)" title="Edit mapping">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none"
                     stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 20h9"/>
                  <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
                </svg>
              </button>
              <button class="btn-delete" @click="removeMapping(tag as string)" title="Delete mapping">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none"
                     stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
          </div>
          <div v-if="Object.keys(tagMappings).length === 0" class="mappings-empty">
            No tags mapped yet. Add a tag redirect mapping below.
          </div>
        </div>

        <div class="add-mapping-form">
          <div class="input-row align-end">
            <div class="input-group size-small">
              <label for="new-tag">{{ editingTag ? 'Editing Tag' : 'Tag' }}</label>
              <input id="new-tag" v-model="newTag" :placeholder="editingTag ? 'edit tag' : 'e.g. sermon'"
                     @keyup.enter="saveMapping"/>
            </div>
            <div class="input-group flex-grow">
              <label for="new-url">Redirect Target URL</label>
              <input id="new-url" v-model="newUrl" placeholder="https://example.com/notes" @keyup.enter="saveMapping"/>
            </div>
          </div>
          <div class="button-group">
            <button class="btn btn-add" @click="saveMapping">
              {{ editingTag ? 'Save' : 'Add' }}
            </button>
            <button v-if="editingTag" class="btn btn-secondary btn-cancel" @click="cancelEdit">Cancel</button>
          </div>
          <p v-if="addError" class="add-error">{{ addError }}</p>
        </div>
      </div>

      <div class="card-actions">
        <button
            :class="['btn', hasUnsavedChanges ? 'btn-warning highlight-btn' : 'btn-primary', 'w-full']"
            @click="emit('push')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
          {{
            hasUnsavedChanges ? 'Push Unsaved Mappings to Cloudflare Worker' : 'Push Tag Mappings to Cloudflare Worker'
          }}
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

.input-row.align-end {
  align-items: flex-end;
}

.flex-grow {
  flex-grow: 1;
}

.size-small {
  flex: 0 0 110px;
}

.w-full {
  width: 100%;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.input-group label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.mappings-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  border-top: 1px solid var(--border-color);
  padding-top: 24px;
  margin-top: 8px;
}

.editor-label {
  font-size: 11px;
  font-weight: 800;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.mappings-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 220px;
  overflow-y: auto;
  background: var(--log-container-bg);
  border-radius: 16px;
  padding: 12px;
  border: 1px solid var(--border-color);
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.02);
}

.mapping-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--card-bg);
  border-radius: 12px;
  border: 1px solid var(--border-color);
  font-size: 13px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.02);
  transition: all 0.2s ease;
}

.mapping-item:hover {
  border-color: var(--primary-color);
  transform: translateX(4px);
}

.mapping-tag {
  font-weight: 800;
  color: var(--primary-color);
  font-family: 'JetBrains Mono', monospace;
}

.mapping-arrow {
  color: var(--text-muted);
  opacity: 0.3;
}

.mapping-url {
  color: var(--text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-grow: 1;
  font-size: 12px;
}

.action-buttons {
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn-delete, .btn-edit {
  background: var(--btn-secondary-bg);
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px;
  border-radius: 8px;
  transition: all 0.2s ease;
}

.btn-delete:hover {
  color: white;
  background: var(--error);
}

.btn-edit:hover {
  color: white;
  background: var(--primary-color);
}

.mappings-empty {
  font-size: 14px;
  color: var(--text-muted);
  font-style: italic;
  text-align: center;
  padding: 40px 20px;
  opacity: 0.6;
}

.add-mapping-form {
  margin-top: 8px;
  background: var(--bg-color);
  border: 2px dashed var(--border-color);
  border-radius: 16px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.button-group {
  display: flex;
  gap: 10px;
}

.btn-add {
  flex: 1;
  background: var(--primary-color);
  color: white;
  border: none;
  height: 40px;
  padding: 0 16px;
  border-radius: 10px;
  cursor: pointer;
  font-weight: 700;
  font-size: 13px;
  transition: all 0.2s ease;
}

.btn-add:hover {
  background: var(--primary-hover);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

.btn-cancel {
  background: var(--btn-secondary-bg);
  color: var(--text-color);
  border: 1px solid var(--border-color);
  height: 40px;
  padding: 0 16px;
  border-radius: 10px;
  cursor: pointer;
  font-weight: 700;
  font-size: 13px;
  transition: all 0.2s ease;
}

.btn-cancel:hover {
  background: var(--border-color);
}

.add-error {
  font-size: 12px;
  color: var(--error);
  margin-top: 4px;
  font-weight: 600;
  text-align: center;
}

.btn-warning {
  background: var(--warning) !important;
  color: white !important;
}

.btn-warning:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(245, 158, 11, 0.3) !important;
}
</style>
