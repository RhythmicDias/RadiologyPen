<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";

  let status = $state("waiting"); // waiting, loading, complete, error
  let resultText = $state("");

  onMount(() => {
    const unlistenStatus = listen<{ status: string, result?: string, error?: string }>("ai-result-update", (event) => {
      status = event.payload.status;
      if (event.payload.result) resultText = event.payload.result;
      if (event.payload.error) resultText = event.payload.error;
    });

    return () => {
      unlistenStatus.then(f => f());
    };
  });

  async function closeWindow() {
    await invoke("hide_ai_result_window");
  }

  function startDrag() {
    invoke("start_drag").catch(err => console.error(err));
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
</script>

<main class="result-container">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="header" onmousedown={startDrag} style="cursor: grab;">
    <h3>AI Analysis</h3>
    <button 
      class="close-btn" 
      onmousedown={(e) => e.stopPropagation()} 
      onclick={closeWindow}
    >✕</button>
  </div>
  
  <div class="content">
    {#if status === "loading"}
      <div class="spinner-container">
        <div class="spinner"></div>
        <p>Analyzing image...</p>
      </div>
    {:else if status === "complete"}
      <div class="markdown-result">
        {@html parseMarkdown(resultText)}
      </div>
    {:else if status === "error"}
      <div class="error-text">Error: {resultText}</div>
    {:else}
      <div class="waiting-text">Waiting for image capture...</div>
    {/if}
  </div>
</main>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap');

  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    font-family: 'Outfit', sans-serif;
  }
  
  .result-container {
    background: rgba(21, 15, 36, 0.96); /* Faded purple background */
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(179, 136, 255, 0.2);
    border-radius: 12px;
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    color: #e5def2;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    box-sizing: border-box;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 15px;
    border-bottom: 1px solid rgba(179, 136, 255, 0.2);
    cursor: grab;
  }

  .header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    pointer-events: none;
    color: #ffffff;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #a095b5;
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    transition: color 0.15s;
  }
  .close-btn:hover {
    color: #b388ff;
  }

  .content {
    flex: 1;
    padding: 15px;
    overflow-y: auto;
  }

  .markdown-result {
    font-size: 14px;
    line-height: 1.6;
  }

  .markdown-result :global(h1), 
  .markdown-result :global(h2), 
  .markdown-result :global(h3), 
  .markdown-result :global(h4), 
  .markdown-result :global(h5) {
    margin-top: 18px;
    margin-bottom: 8px;
    color: #b388ff;
    font-weight: 600;
  }
  .markdown-result :global(h1) { font-size: 18px; }
  .markdown-result :global(h2) { font-size: 16px; }
  .markdown-result :global(h3) { font-size: 15px; }
  .markdown-result :global(h4) { font-size: 14px; }
  .markdown-result :global(p) {
    margin: 8px 0;
    font-size: 13.5px;
  }
  .markdown-result :global(ul) {
    margin: 6px 0 12px 0;
    padding-left: 20px;
  }
  .markdown-result :global(li) {
    margin: 4px 0;
    font-size: 13.5px;
  }
  .markdown-result :global(strong) {
    color: #ffffff;
    font-weight: 600;
  }

  .spinner-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #a095b5;
  }

  .spinner {
    border: 3px solid rgba(179, 136, 255, 0.1);
    border-top: 3px solid #b388ff;
    border-radius: 50%;
    width: 24px;
    height: 24px;
    animation: spin 1s linear infinite;
    margin-bottom: 10px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .error-text {
    color: #ff453a;
    font-size: 14px;
  }
  
  .waiting-text {
    color: #a095b5;
    text-align: center;
    margin-top: 20px;
    font-style: italic;
  }
</style>
