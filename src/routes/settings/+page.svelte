<script lang="ts">
  import { settingsStore } from "$lib/settingsStore";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";

  let activeTab = $state("config");
  
  let tempKey = $state("");
  let tempModel = $state("");
  let saveMessage = $state("");
  let isLoadingModels = $state(false);
  let searchQuery = $state("");
  let isDropdownOpen = $state(false);
  let dropdownRef = $state<HTMLElement | null>(null);
  
  interface OpenRouterModel {
    id: string;
    name: string;
  }

  // Hardcoded fallback models if fetching fails (now including free VL models)
  const defaultModels: Record<string, string[]> = {
    "Google": ["google/gemini-flash-1.5", "google/gemini-pro-1.5"],
    "OpenAI": ["openai/gpt-4o", "openai/gpt-4o-mini"],
    "Anthropic": ["anthropic/claude-3.5-sonnet", "anthropic/claude-3-opus"],
    "Meta (Free)": ["meta-llama/llama-3.2-11b-vision-instruct:free"],
    "Qwen (Free)": ["qwen/qwen-2-vl-7b-instruct:free"],
    "Mistral (Free)": ["mistralai/pixtral-12b:free"]
  };
  
  let fetchedModels = $state<OpenRouterModel[]>([]);

  function formatProviderName(rawProvider: string): string {
    const mapping: Record<string, string> = {
      "openai": "OpenAI",
      "anthropic": "Anthropic",
      "google": "Google",
      "meta-llama": "Meta",
      "mistralai": "Mistral",
      "qwen": "Qwen",
      "microsoft": "Microsoft",
      "nvidia": "Nvidia",
      "cohere": "Cohere"
    };
    return mapping[rawProvider.toLowerCase()] || (rawProvider.charAt(0).toUpperCase() + rawProvider.slice(1));
  }
  
  // Group models by provider (the prefix before the slash)
  let groupedModels = $derived.by(() => {
    const groups: Record<string, OpenRouterModel[]> = {};
    
    if (fetchedModels.length > 0) {
      for (const m of fetchedModels) {
        const parts = m.id.split('/');
        const rawProvider = parts.length > 1 ? parts[0] : 'other';
        const provider = formatProviderName(rawProvider);
        if (!groups[provider]) {
          groups[provider] = [];
        }
        groups[provider].push(m);
      }
    } else {
      // Fallback groupings
      for (const [provider, ids] of Object.entries(defaultModels)) {
        groups[provider] = ids.map(id => ({
          id,
          name: id.split('/')[1].replace(/-/g, ' ').toUpperCase()
        }));
      }
    }
    
    // Sort keys and sub-lists
    const sorted: Record<string, OpenRouterModel[]> = {};
    Object.keys(groups).sort().forEach(k => {
      sorted[k] = groups[k].sort((a, b) => a.name.localeCompare(b.name));
    });
    return sorted;
  });

  // Filter grouped models by search query
  let filteredGroupedModels = $derived.by(() => {
    const query = searchQuery.toLowerCase().trim();
    if (!query) return groupedModels;

    const result: Record<string, OpenRouterModel[]> = {};
    for (const [provider, models] of Object.entries(groupedModels)) {
      const filtered = models.filter(m => 
        m.name.toLowerCase().includes(query) || 
        m.id.toLowerCase().includes(query)
      );
      if (filtered.length > 0) {
        result[provider] = filtered;
      }
    }
    return result;
  });

  // Find formatted name of currently selected model
  let selectedModelName = $derived.by(() => {
    const allModels = Object.values(groupedModels).flat();
    const found = allModels.find(m => m.id === tempModel);
    return found ? found.name : tempModel;
  });
  
  async function fetchModels() {
    isLoadingModels = true;
    try {
      // Fetch models from OpenRouter that support image input modality
      const res = await fetch("https://openrouter.ai/api/v1/models?input_modalities=image");
      if (res.ok) {
        const data = await res.json();
        if (data && Array.isArray(data.data)) {
          fetchedModels = data.data.map((m: any) => ({
            id: m.id,
            name: m.name || m.id
          }));
        }
      }
    } catch (e) {
      console.error("Failed to fetch OpenRouter models:", e);
    } finally {
      isLoadingModels = false;
    }
  }

  function applyChanges() {
    settingsStore.update(s => {
      return {
        ...s,
        model: tempModel,
        apiKeys: {
          ...s.apiKeys,
          "OpenRouter": tempKey
        }
      };
    });
    saveMessage = "Configuration applied successfully!";
    setTimeout(() => saveMessage = "", 3000);
  }

  function handleOutsideClick(e: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
      isDropdownOpen = false;
    }
  }

  function selectModel(id: string) {
    tempModel = id;
    isDropdownOpen = false;
    searchQuery = ""; // Reset search query on selection
  }

  onMount(() => {
    if ($settingsStore.provider !== "OpenRouter") {
      settingsStore.update(s => ({ ...s, provider: "OpenRouter" }));
    }
    tempKey = $settingsStore.apiKeys["OpenRouter"] || "";
    tempModel = $settingsStore.model || "google/gemini-flash-1.5";
    fetchModels();

    window.addEventListener("click", handleOutsideClick);
    
    // Sync store on window focus or storage changes
    const handleStorage = () => {
      settingsStore.load();
      tempKey = $settingsStore.apiKeys["OpenRouter"] || "";
      tempModel = $settingsStore.model || "";
    };
    window.addEventListener("storage", handleStorage);

    const unlistenFocus = listen("tauri://focus", () => {
      settingsStore.load();
      tempKey = $settingsStore.apiKeys["OpenRouter"] || "";
      tempModel = $settingsStore.model || "";
    });

    return () => {
      window.removeEventListener("click", handleOutsideClick);
      window.removeEventListener("storage", handleStorage);
      unlistenFocus.then(f => f());
    };
  });

  function resetCount() {
    if (confirm("Are you sure you want to reset the AI analysis counter?")) {
      settingsStore.clearCount();
    }
  }

  async function clearHistory() {
    if (confirm("Are you sure you want to clear all analysis history?")) {
      try {
        await invoke("clear_history_images");
      } catch (err) {
        console.error("Failed to delete history images from disk:", err);
      }
      settingsStore.clearHistory();
    }
  }

  function parseMarkdown(md: string): string {
    if (!md) return "";
    let html = md
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

    // Headings
    html = html.replace(/^##### (.*?)$/gm, "<h5>$1</h5>");
    html = html.replace(/^#### (.*?)$/gm, "<h4>$1</h4>");
    html = html.replace(/^### (.*?)$/gm, "<h3>$1</h3>");
    html = html.replace(/^## (.*?)$/gm, "<h2>$1</h2>");
    html = html.replace(/^# (.*?)$/gm, "<h1>$1</h1>");

    // Bold
    html = html.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>");
    
    // Italic
    html = html.replace(/\*(.*?)\*/g, "<em>$1</em>");

    // Bullet points
    html = html.replace(/^\s*[-*]\s+(.*?)$/gm, "<li>$1</li>");

    // Wrap consecutive list items in ul
    const lines = html.split("\n");
    let inList = false;
    const processed = [];
    for (const line of lines) {
      const isLi = line.includes("<li>");
      if (isLi && !inList) {
        processed.push("<ul>");
        inList = true;
      } else if (!isLi && inList) {
        processed.push("</ul>");
        inList = false;
      }
      processed.push(line);
    }
    if (inList) processed.push("</ul>");

    return processed.join("\n");
  }

  function formatModelId(id: string): string {
    if (!id) return "";
    const parts = id.split('/');
    const name = parts.length > 1 ? parts[1] : id;
    return name.replace(/:free$/, ' (free)').replace(/-/g, ' ').toUpperCase();
  }
</script>

<main class="settings-container">
  <div class="tabs">
    <button class="tab-btn" class:active={activeTab === "config"} onclick={() => activeTab = "config"}>Ai Configuration</button>
    <button class="tab-btn" class:active={activeTab === "history"} onclick={() => activeTab = "history"}>Ai Analysis History</button>
  </div>

  {#if activeTab === "config"}
    <div class="config-panel">
      <div class="form-group">
        <label>Ai Provider</label>
        <div class="read-only-field">OpenRouter (Single Key)</div>
      </div>

      <div class="form-group">
        <label>AI Model (Visual / Vision Capable)</label>
        
        <!-- Custom Searchable Dropdown -->
        <div class="custom-select-container" bind:this={dropdownRef}>
          <button class="select-trigger" onclick={() => isDropdownOpen = !isDropdownOpen}>
            <span>{selectedModelName || "Select a model..."}</span>
            <span class="arrow">{isDropdownOpen ? '▲' : '▼'}</span>
          </button>
          
          {#if isDropdownOpen}
            <div class="dropdown-popover">
              <input 
                type="text" 
                bind:value={searchQuery} 
                placeholder="🔍 Type to filter models (e.g. gemini, free)..." 
                class="popover-search"
                onclick={(e) => e.stopPropagation()} 
                autofocus
              />
              <div class="options-list">
                {#each Object.entries(filteredGroupedModels) as [provider, models]}
                  <div class="provider-header">{provider}</div>
                  {#each models as m}
                    <button 
                      class="option-item" 
                      class:selected={tempModel === m.id}
                      onclick={() => selectModel(m.id)}
                    >
                      <span class="model-name">{m.name}</span>
                      <span class="model-id">{m.id}</span>
                    </button>
                  {/each}
                {:else}
                  <div class="no-results">No vision models found</div>
                {/each}
              </div>
            </div>
          {/if}
        </div>

        <small class="hint">
          {#if isLoadingModels}
            Fetching visual models...
          {:else if fetchedModels.length > 0}
            Displaying vision-capable models fetched dynamically from OpenRouter.
          {:else}
            Showing fallback visual models.
          {/if}
        </small>
      </div>

      <div class="form-group">
        <label>Set OpenRouter API Key</label>
        <div class="api-key-input">
          <input type="password" bind:value={tempKey} placeholder="sk-or-v1-..." />
        </div>
      </div>

      <div class="apply-actions">
        <button class="primary-btn apply-btn" onclick={applyChanges}>Apply Changes</button>
        {#if saveMessage}
          <div class="save-message">{saveMessage}</div>
        {/if}
      </div>
    </div>
  {:else if activeTab === "history"}
    <div class="history-panel">
      <div class="history-header">
        <div class="stat-box">
          <span class="stat-label">Total AI Analyses Done:</span>
          <span class="stat-value">{$settingsStore.totalCount || 0}</span>
        </div>
        <div class="action-buttons">
          <button class="secondary-btn" onclick={resetCount}>Reset Count</button>
          <button class="danger-btn" onclick={clearHistory} disabled={$settingsStore.history.length === 0}>Clear History</button>
        </div>
      </div>

      {#if $settingsStore.history.length === 0}
        <div class="empty-state">No analysis history found. Use the AI tool on the canvas to start!</div>
      {:else}
        {#each $settingsStore.history as item}
          <div class="history-item">
            <div class="time-model-row">
              <span class="time">{new Date(item.timestamp).toLocaleString()}</span>
              {#if item.model}
                <span class="model-badge">{formatModelId(item.model)}</span>
              {/if}
            </div>
            {#if item.imageUrl}
               <img src={item.imageUrl.startsWith("data:") ? item.imageUrl : convertFileSrc(item.imageUrl)} alt="Cropped area" />
            {/if}
            <div class="result markdown-result">{@html parseMarkdown(item.result)}</div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</main>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap');

  input, select, button, textarea {
    font-family: 'Outfit', sans-serif !important;
  }

  .settings-container {
    padding: 20px;
    color: #e5def2;
    background: rgba(21, 15, 36, 0.98); /* Faded purple background */
    height: 100vh;
    box-sizing: border-box;
    font-family: 'Outfit', sans-serif;
    overflow-y: auto;
  }
  .tabs {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
    border-bottom: 1px solid rgba(179, 136, 255, 0.2);
    padding-bottom: 10px;
  }
  .tab-btn {
    background: transparent;
    border: none;
    color: #a095b5;
    cursor: pointer;
    font-size: 16px;
    padding: 5px 10px;
    transition: color 0.15s;
  }
  .tab-btn:hover {
    color: #b388ff;
  }
  .tab-btn.active {
    color: #b388ff;
    font-weight: bold;
    border-bottom: 2px solid #b388ff;
  }
  .form-group {
    margin-bottom: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  label {
    font-weight: 500;
    color: #ffffff;
  }
  .read-only-field {
    background: rgba(179, 136, 255, 0.05);
    border: 1px solid rgba(179, 136, 255, 0.15);
    color: #a095b5;
    padding: 10px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
  }
  
  /* Custom Select Dropdown styles */
  .custom-select-container {
    position: relative;
    width: 100%;
  }
  .select-trigger {
    background: rgba(179, 136, 255, 0.05);
    border: 1px solid rgba(179, 136, 255, 0.15);
    color: #ffffff;
    padding: 12px;
    border-radius: 8px;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }
  .select-trigger:hover, .select-trigger:focus {
    border-color: #b388ff;
  }
  .select-trigger .arrow {
    font-size: 10px;
    color: #a095b5;
  }
  .dropdown-popover {
    position: absolute;
    top: calc(100% + 5px);
    left: 0;
    width: 100%;
    background: rgba(21, 15, 36, 0.98);
    border: 1px solid rgba(179, 136, 255, 0.25);
    border-radius: 8px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    max-height: 250px;
    box-sizing: border-box;
    overflow: hidden;
  }
  .popover-search {
    background: rgba(179, 136, 255, 0.08);
    border: none;
    border-bottom: 1px solid rgba(179, 136, 255, 0.2);
    color: #ffffff;
    padding: 10px 12px;
    font-size: 13px;
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }
  .options-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }
  .provider-header {
    font-size: 10px;
    text-transform: uppercase;
    font-weight: 750;
    color: #b388ff;
    padding: 6px 8px 3px 8px;
    letter-spacing: 0.5px;
  }
  .option-item {
    background: transparent;
    border: none;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    box-sizing: border-box;
    transition: background 0.15s;
  }
  .option-item:hover {
    background: rgba(179, 136, 255, 0.1);
  }
  .option-item.selected {
    background: rgba(179, 136, 255, 0.15);
  }
  .option-item.selected .model-name {
    color: #b388ff;
    font-weight: 600;
  }
  .model-name {
    color: #ffffff;
    font-size: 13px;
    font-weight: 500;
  }
  .model-id {
    color: #a095b5;
    font-size: 10.5px;
    font-family: monospace;
  }
  .no-results {
    color: #a095b5;
    text-align: center;
    padding: 20px 0;
    font-size: 13px;
  }
  
  input[type="password"] {
    background: rgba(179, 136, 255, 0.05);
    border: 1px solid rgba(179, 136, 255, 0.15);
    color: #ffffff;
    padding: 10px;
    border-radius: 8px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;
  }
  input[type="password"]:focus {
    border-color: #b388ff;
  }
  .api-key-input {
    display: flex;
    gap: 10px;
  }
  .api-key-input input {
    flex: 1;
  }
  .primary-btn {
    background: #b388ff;
    color: #150f24;
    border: none;
    padding: 10px 15px;
    border-radius: 8px;
    cursor: pointer;
    font-weight: bold;
    transition: background 0.15s;
  }
  .primary-btn:hover {
    background: #9e6eff;
  }
  .hint {
    color: #a095b5;
    font-size: 12px;
    margin-top: 5px;
    display: inline-block;
  }
  .save-message {
    color: #34c759;
    font-size: 13px;
    margin-top: 5px;
  }
  .history-panel {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .history-item {
    background: rgba(179, 136, 255, 0.03);
    border: 1px solid rgba(179, 136, 255, 0.12);
    padding: 15px;
    border-radius: 12px;
  }
  .history-item img {
    max-height: 150px;
    margin: 10px 0;
    border-radius: 6px;
    border: 1px solid rgba(179, 136, 255, 0.2);
  }
  .time {
    color: #a095b5;
    font-size: 12px;
  }
  .empty-state {
    color: #a095b5;
    text-align: center;
    padding: 40px 0;
  }

  /* History Stats Header */
  .history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(179, 136, 255, 0.05);
    border: 1px solid rgba(179, 136, 255, 0.15);
    border-radius: 12px;
    padding: 12px 18px;
    margin-bottom: 15px;
    box-sizing: border-box;
  }
  .stat-box {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stat-label {
    font-size: 14px;
    color: #a095b5;
  }
  .stat-value {
    font-size: 16px;
    font-weight: 700;
    color: #b388ff;
  }
  .action-buttons {
    display: flex;
    gap: 10px;
  }
  .secondary-btn {
    background: transparent;
    border: 1px solid rgba(179, 136, 255, 0.3);
    color: #b388ff;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    transition: all 0.15s;
  }
  .secondary-btn:hover {
    background: rgba(179, 136, 255, 0.1);
    border-color: #b388ff;
  }
  .danger-btn {
    background: rgba(255, 69, 58, 0.1);
    border: 1px solid rgba(255, 69, 58, 0.3);
    color: #ff453a;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    transition: all 0.15s;
  }
  .danger-btn:hover:not(:disabled) {
    background: rgba(255, 69, 58, 0.2);
    border-color: #ff453a;
  }
  .danger-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Markdown output inside history items */
  .markdown-result {
    font-size: 13.5px;
    line-height: 1.5;
    margin-top: 10px;
  }
  .markdown-result :global(h1), 
  .markdown-result :global(h2), 
  .markdown-result :global(h3), 
  .markdown-result :global(h4), 
  .markdown-result :global(h5) {
    margin-top: 14px;
    margin-bottom: 6px;
    color: #b388ff;
    font-weight: 600;
  }
  .markdown-result :global(h1) { font-size: 16px; }
  .markdown-result :global(h2) { font-size: 14px; }
  .markdown-result :global(h3) { font-size: 13.5px; }
  .markdown-result :global(h4) { font-size: 13px; }
  .markdown-result :global(p) {
    margin: 6px 0;
  }
  .markdown-result :global(ul) {
    margin: 4px 0 8px 0;
    padding-left: 18px;
  }
  .markdown-result :global(li) {
    margin: 3px 0;
  }
  .markdown-result :global(strong) {
    color: #ffffff;
    font-weight: 600;
  }

  .time-model-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  .model-badge {
    background: rgba(179, 136, 255, 0.1);
    border: 1px solid rgba(179, 136, 255, 0.25);
    color: #b388ff;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.5px;
  }
</style>
