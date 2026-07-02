<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emit, listen } from "@tauri-apps/api/event";
  import { register } from "@tauri-apps/plugin-global-shortcut";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { settingsStore } from "$lib/settingsStore";
  import { analyzeImage } from "$lib/aiService";
  import { onMount } from "svelte";

  function startDrag() {
    invoke("start_drag").catch(err => console.error(err));
  }

  let isMirrorActive = $state(false);
  let activeTool = $state("none");
  let selectedColor = $state("#ff3b30"); // default red
  
  // AI Status tracking
  let aiStatus = $state<"idle" | "capturing" | "processing" | "success" | "error">("idle");
  let aiStatusMessage = $state("");

  const colors = [
    { name: "Red", value: "#ff3b30" },
    { name: "Yellow", value: "#ffcc00" },
    { name: "Green", value: "#34c759" },
    { name: "Blue", value: "#007aff" }
  ];

  async function checkMirrorStatus() {
    isMirrorActive = await invoke("is_mirror_active");
  }

  onMount(() => {
    checkMirrorStatus();

    // Register F8 to toggle annotation overlay globally
    const registerShortcut = async () => {
      try {
        await register("F8", async (event) => {
          if (event.state === "Pressed") {
            if (activeTool !== "none") {
              activeTool = "none";
              await invoke("hide_canvas_window");
            } else {
              await triggerAnnotation("box");
            }
          }
        });
      } catch (err) {
        console.error("Failed to register global F8 shortcut:", err);
      }
    };
    registerShortcut();

    // Listen for canvas window close to reset active tool
    const unlistenClose = listen("canvas-closed", () => {
      activeTool = "none";
      if (aiStatus === "capturing") {
        aiStatus = "idle";
        aiStatusMessage = "";
      }
    });

    function base64ToBytes(base64: string): Uint8Array {
      const parts = base64.split(",");
      const binaryStr = atob(parts.length > 1 ? parts[1] : parts[0]);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
      }
      return bytes;
    }

    // AI logic orchestration
    const unlistenAi = listen<{ image: string }>("ai-analyze-image", async (event) => {
      aiStatus = "processing";
      aiStatusMessage = "Analyzing captured region...";
      try {
        await invoke("show_ai_result_window");
        await emit("ai-result-update", { status: "loading" });
        
        let imagePath = "";
        try {
          const bytes = base64ToBytes(event.payload.image);
          imagePath = await invoke<string>("save_history_image", { id: Date.now().toString(), bytes: Array.from(bytes) });
        } catch (err) {
          console.error("Failed to save history image to disk:", err);
          imagePath = event.payload.image; // Fallback to base64 if disk save fails
        }
        
        let streamedResult = "";
        const text = await analyzeImage(event.payload.image, async (chunkText) => {
          streamedResult = chunkText;
          await emit("ai-result-update", { status: "complete", result: chunkText });
        });
        
        aiStatus = "success";
        aiStatusMessage = "Analysis complete!";
        await invoke("show_ai_result_window"); // Re-open window if it was closed during processing
        await emit("ai-result-update", { status: "complete", result: text });
        settingsStore.addHistory(imagePath, text);
        
        setTimeout(() => {
          if (aiStatus === "success") {
            aiStatus = "idle";
            aiStatusMessage = "";
          }
        }, 3000);
        
      } catch (e: any) {
        aiStatus = "error";
        aiStatusMessage = "AI Error: " + (e.message || String(e));
        await emit("ai-result-update", { status: "error", error: e.message || String(e) });
        
        setTimeout(() => {
          if (aiStatus === "error") {
            aiStatus = "idle";
            aiStatusMessage = "";
          }
        }, 5000);
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "1") triggerAnnotation("box");
      else if (e.key === "2") triggerAnnotation("circle");
      else if (e.key === "3") triggerAnnotation("arrow");
      else if (e.key === "4") triggerAnnotation("magnifier");
      else if (e.key === "5") triggerAnnotation("cursor");
      else if (e.key === "6") triggerAnnotation("ai-crop");
      else if (e.key.toLowerCase() === "c") clearAnnotations();
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlistenClose.then(f => f());
      unlistenAi.then(f => f());
      window.removeEventListener("keydown", handleKeyDown);
    };
  });

  async function triggerAnnotation(toolName: string) {
    if (toolName === activeTool) {
      // Toggle off
      activeTool = "none";
      await invoke("hide_canvas_window");
      return;
    }

    activeTool = toolName;
    
    if (toolName === "ai-crop") {
      aiStatus = "capturing";
      aiStatusMessage = "Drag a rectangle to crop and analyze...";
    }
    
    // Take screenshot and open canvas window
    try {
      const imgBytes = await invoke<number[]>("capture_screen");
      // Send image bytes to canvas window
      await emit("screenshot-captured", { bytes: imgBytes });
      // Tell canvas window about the active tool and color
      await emit("tool-changed", { tool: toolName, color: selectedColor });
      // Show canvas window
      await invoke("show_canvas_window");
    } catch (err) {
      console.error("Capture failed:", err);
      activeTool = "none";
      aiStatus = "error";
      aiStatusMessage = "Screen capture failed.";
      setTimeout(() => {
        if (aiStatus === "error") {
          aiStatus = "idle";
          aiStatusMessage = "";
        }
      }, 4000);
    }
  }

  async function toggleMirror() {
    if (isMirrorActive) {
      await invoke("hide_mirror_window");
      isMirrorActive = false;
    } else {
      await invoke("show_mirror_window");
      isMirrorActive = true;
    }
    await emit("mirror-toggled", { active: isMirrorActive });
  }

  async function selectColor(color: string) {
    selectedColor = color;
    await emit("color-changed", { color });
  }

  async function clearAnnotations() {
    await emit("clear-annotations");
  }

  async function undoLast() {
    await emit("undo-annotation");
  }
</script>

<!-- The data-tauri-drag-region makes the entire background draggable -->
<div class="window-wrapper">
  <div class="main-layout">
    <main class="toolbar-container" data-tauri-drag-region>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="drag-handle" onmousedown={startDrag} style="cursor: grab;">
        <!-- Drag Dots -->
        <svg width="12" height="20" viewBox="0 0 12 20" fill="none" class="drag-icon">
          <circle cx="3" cy="3" r="2" fill="currentColor"/>
          <circle cx="3" cy="10" r="2" fill="currentColor"/>
          <circle cx="3" cy="17" r="2" fill="currentColor"/>
          <circle cx="9" cy="3" r="2" fill="currentColor"/>
          <circle cx="9" cy="10" r="2" fill="currentColor"/>
          <circle cx="9" cy="17" r="2" fill="currentColor"/>
        </svg>
      </div>

      <!-- Drawing Tools -->
      <div class="tools-group">
        <!-- Box Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">1</span>
          <button 
            class="tool-btn" 
            class:active={activeTool === "box"} 
            onclick={() => triggerAnnotation("box")}
            title="Rectangle Tool (1)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2" />
            </svg>
          </button>
        </div>

        <!-- Circle Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">2</span>
          <button 
            class="tool-btn" 
            class:active={activeTool === "circle"} 
            onclick={() => triggerAnnotation("circle")}
            title="Circle Tool (2)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="9" />
            </svg>
          </button>
        </div>

        <!-- Arrow Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">3</span>
          <button 
            class="tool-btn" 
            class:active={activeTool === "arrow"} 
            onclick={() => triggerAnnotation("arrow")}
            title="Arrow Tool (3)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="5" y1="19" x2="19" y2="5" />
              <polyline points="12 5 19 5 19 12" />
            </svg>
          </button>
        </div>

        <!-- Magnifier Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">4</span>
          <button 
            class="tool-btn" 
            class:active={activeTool === "magnifier"} 
            onclick={() => triggerAnnotation("magnifier")}
            title="Magnifying Glass (4)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
          </button>
        </div>

        <!-- Big Cursor Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">5</span>
          <button 
            class="tool-btn" 
            class:active={activeTool === "cursor"} 
            onclick={() => triggerAnnotation("cursor")}
            title="Highlight Cursor (5)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="none">
              <path d="M4 2 L4 17 L7.5 13.5 L10.5 20 L12.5 19 L9.5 12 L14 12 Z" />
            </svg>
          </button>
        </div>

        <div class="divider"></div>

        <!-- AI Tool -->
        <div class="tool-wrapper">
          <span class="shortcut-number">6</span>
          <button 
            class="tool-btn ai-btn" 
            class:active={activeTool === "ai-crop"} 
            onclick={() => triggerAnnotation("ai-crop")}
            title="AI Screenshot Analysis (6)"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
            </svg>
          </button>
        </div>
      </div>

      <div class="divider"></div>

      <!-- Color Swatches -->
      <div class="colors-group">
        {#each colors as color}
          <button 
            class="color-btn" 
            style="background-color: {color.value}"
            class:selected={selectedColor === color.value}
            onclick={() => selectColor(color.value)}
            title={color.name}
          >
          </button>
        {/each}
      </div>

      <div class="divider"></div>

      <!-- Utility / System Controls -->
      <div class="utilities-group">
        <!-- Undo Button -->
        <button class="util-btn" onclick={undoLast} title="Undo (Ctrl+Z)" disabled={activeTool === "none"}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 7v6h6" />
            <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
          </svg>
        </button>

        <!-- Clear Button -->
        <button class="util-btn" onclick={clearAnnotations} title="Clear All" disabled={activeTool === "none"}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 6h18" />
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
          </svg>
        </button>

        <!-- Mirror Screen Toggle -->
        <button 
          class="util-btn" 
          class:active={isMirrorActive} 
          onclick={toggleMirror}
          title="Toggle Parent Display Mirror"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
        </button>
      </div>
    </main>

    {#if aiStatusMessage}
      <div class="ai-status-indicator" class:error={aiStatus === 'error'} class:success={aiStatus === 'success'}>
        {#if aiStatus === 'processing' || aiStatus === 'capturing'}
          <div class="status-spinner"></div>
        {/if}
        <span>{aiStatusMessage}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Window wrapper: position:fixed fills window without causing overflow/scrollbars */
  .window-wrapper {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    background: transparent;  /* explicit transparent — no WebView2 fill */
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    overflow: hidden;
  }

  .main-layout {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: auto;
  }

  .toolbar-container {
    display: flex;
    align-items: center;
    background: rgba(28, 28, 30, 0.92);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 40px;
    padding: 6px 14px;
    color: #ffffff;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.25);
    user-select: none;
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .drag-handle {
    cursor: grab;
    display: flex;
    align-items: center;
    padding-right: 4px;
    color: rgba(255, 255, 255, 0.4);
    transition: color 0.2s;
  }

  .drag-handle:hover {
    color: rgba(255, 255, 255, 0.8);
  }

  .drag-icon {
    flex-shrink: 0;
  }

  .divider {
    width: 1px;
    height: 24px;
    background: rgba(255, 255, 255, 0.15);
    margin: 0 10px;
  }

  .tools-group, .colors-group, .utilities-group {
    display: flex;
    align-items: flex-end; /* Align to bottom so buttons line up if shortcut numbers add height */
    gap: 6px;
  }

  .tool-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .shortcut-number {
    font-size: 10px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.4);
    line-height: 1;
    user-select: none;
  }

  .tool-btn, .util-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.7);
    padding: 6px;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    outline: none;
  }

  .tool-btn:hover:not(:disabled), .util-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
    transform: scale(1.08);
  }

  .tool-btn:active:not(:disabled), .util-btn:active:not(:disabled) {
    transform: scale(0.95);
  }

  .tool-btn.active {
    background: #007aff;
    color: #ffffff;
    box-shadow: 0 0 12px rgba(0, 122, 255, 0.4);
  }

  .util-btn.active {
    color: #34c759;
  }

  .util-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .ai-btn {
    color: #b388ff;
  }

  .ai-btn.active {
    background: #b388ff;
    color: #150f24;
    box-shadow: 0 0 12px rgba(179, 136, 255, 0.4);
  }

  .color-btn {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    position: relative;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s;
    outline: none;
    box-shadow: none;
  }

  .color-btn:hover {
    transform: scale(1.2);
  }

  .color-btn.selected {
    transform: scale(1.15);
    outline: 1.5px solid rgba(255, 255, 255, 0.9);
    outline-offset: 2px;
  }

  /* AI Status Indicator Styles */
  .ai-status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(21, 15, 36, 0.9);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(179, 136, 255, 0.35);
    color: #b388ff;
    padding: 6px 14px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 600;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.25);
    animation: fadeIn 0.25s ease-out;
    pointer-events: none;
    user-select: none;
  }

  .ai-status-indicator.error {
    border-color: rgba(255, 59, 48, 0.35);
    color: #ff453a;
  }

  .ai-status-indicator.success {
    border-color: rgba(52, 199, 89, 0.35);
    color: #30d158;
  }

  .status-spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid rgba(179, 136, 255, 0.2);
    border-top: 1.5px solid #b388ff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(-5px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
