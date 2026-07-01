<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, emit } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface Shape {
    type: "box" | "circle" | "arrow";
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    color: string;
  }

  let screenshotUrl = $state<string | null>(null);
  let screenshotElement = $state<HTMLImageElement | null>(null);
  
  let activeTool = $state("none");
  let activeColor = $state("#ff3b30");
  let annotations = $state<Shape[]>([]);
  let tempShape = $state<Shape | null>(null);
  let isDrawing = $state(false);

  // Magnifier configuration
  let zoomLevel = $state(3); // Default 3x zoom
  let magnifierSize = $state(350); // Width/height of loupe (increased for more area)

  // Cursor/Hover tracking
  let mouseX = $state(0);
  let mouseY = $state(0);
  let isMouseInCanvas = $state(false);
  let showAnnotations = $state(true); // Toggled by holding Space

  let mainCanvas = $state<HTMLCanvasElement | null>(null);

  // Set up event listeners from Tauri
  onMount(() => {
    // Listen for incoming screenshot bytes
    const unlistenScreenshot = listen<{ bytes: number[] }>("screenshot-captured", (event) => {
      const bytes = event.payload.bytes;
      const blob = new Blob([new Uint8Array(bytes)], { type: "image/png" });
      if (screenshotUrl) {
        URL.revokeObjectURL(screenshotUrl);
      }
      screenshotUrl = URL.createObjectURL(blob);
    });

    // Listen for tool changes from toolbar
    const unlistenTool = listen<{ tool: string; color: string }>("tool-changed", (event) => {
      activeTool = event.payload.tool;
      activeColor = event.payload.color;
    });

    // Listen for color changes
    const unlistenColor = listen<{ color: string }>("color-changed", (event) => {
      activeColor = event.payload.color;
    });

    // Listen for undo
    const unlistenUndo = listen("undo-annotation", () => {
      annotations.pop();
      syncWithMirror();
    });

    // Listen for clear
    const unlistenClear = listen("clear-annotations", () => {
      annotations = [];
      syncWithMirror();
    });

    // Listen to key events for shortcuts
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        closeCanvas();
      } else if (e.key === " ") {
        showAnnotations = false;
        syncWithMirror();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        annotations.pop();
        syncWithMirror();
      } else if (e.key === "Delete" || e.key.toLowerCase() === "c") {
        annotations = [];
        syncWithMirror();
      } else if (e.key === "1") {
        if (activeTool === "box") closeCanvas();
        else { activeTool = "box"; emitToolChange(); }
      } else if (e.key === "2") {
        if (activeTool === "circle") closeCanvas();
        else { activeTool = "circle"; emitToolChange(); }
      } else if (e.key === "3") {
        if (activeTool === "arrow") closeCanvas();
        else { activeTool = "arrow"; emitToolChange(); }
      } else if (e.key === "4") {
        if (activeTool === "magnifier") closeCanvas();
        else { activeTool = "magnifier"; emitToolChange(); }
      } else if (e.key === "5") {
        if (activeTool === "cursor") closeCanvas();
        else { activeTool = "cursor"; emitToolChange(); }
      } else if (e.key === "[" && activeTool === "magnifier") {
        zoomLevel = Math.max(1.5, zoomLevel - 0.5);
      } else if (e.key === "]" && activeTool === "magnifier") {
        zoomLevel = Math.min(8, zoomLevel + 0.5);
      } else if (e.key === "{" && activeTool === "magnifier") {
        magnifierSize = Math.max(120, magnifierSize - 20);
      } else if (e.key === "}" && activeTool === "magnifier") {
        magnifierSize = Math.min(600, magnifierSize + 20);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === " ") {
        showAnnotations = true;
        syncWithMirror();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      unlistenScreenshot.then(f => f());
      unlistenTool.then(f => f());
      unlistenColor.then(f => f());
      unlistenUndo.then(f => f());
      unlistenClear.then(f => f());
    };
  });

  function emitToolChange() {
    emit("tool-changed", { tool: activeTool, color: activeColor });
  }

  function emitColorChange() {
    emit("color-changed", { color: activeColor });
  }

  async function closeCanvas() {
    activeTool = "none";
    await invoke("hide_canvas_window");
    await emit("canvas-closed");
  }

  function syncWithMirror() {
    emit("mirror-sync", {
      annotations,
      tempShape,
      showAnnotations,
      activeTool,
      activeColor,
      mouseX,
      mouseY,
      zoomLevel,
      magnifierSize,
      isMouseInCanvas
    });
  }

  // Draw loop
  $effect(() => {
    if (!mainCanvas || !screenshotElement) return;
    const ctx = mainCanvas.getContext("2d");
    if (!ctx) return;

    // Set canvas dimensions to window size
    mainCanvas.width = window.innerWidth;
    mainCanvas.height = window.innerHeight;

    ctx.clearRect(0, 0, mainCanvas.width, mainCanvas.height);

    // Draw background screenshot
    ctx.drawImage(screenshotElement, 0, 0, mainCanvas.width, mainCanvas.height);

    // Draw saved annotations
    if (showAnnotations) {
      annotations.forEach(shape => drawShape(ctx, shape));
    }

    // Draw active drawing shape
    if (isDrawing && tempShape) {
      drawShape(ctx, tempShape);
    }

    // Draw Magnifier Loupe if in magnifier mode
    if (activeTool === "magnifier" && isMouseInCanvas) {
      drawMagnifier(ctx);
    }

    // Draw Big Cursor if in cursor mode
    if (activeTool === "cursor" && isMouseInCanvas) {
      drawBigCursor(ctx);
    }
  });

  function drawShape(ctx: CanvasRenderingContext2D, shape: Shape) {
    ctx.strokeStyle = shape.color;
    ctx.lineWidth = 3;
    ctx.lineCap = "round";

    if (shape.type === "box") {
      const x = Math.min(shape.x1, shape.x2);
      const y = Math.min(shape.y1, shape.y2);
      const w = Math.abs(shape.x2 - shape.x1);
      const h = Math.abs(shape.y2 - shape.y1);
      ctx.strokeRect(x, y, w, h);
    } else if (shape.type === "circle") {
      const cx = (shape.x1 + shape.x2) / 2;
      const cy = (shape.y1 + shape.y2) / 2;
      const rx = Math.abs(shape.x2 - shape.x1) / 2;
      const ry = Math.abs(shape.y2 - shape.y1) / 2;
      ctx.beginPath();
      ctx.ellipse(cx, cy, rx, ry, 0, 0, 2 * Math.PI);
      ctx.stroke();
    } else if (shape.type === "arrow") {
      // Draw Arrow stem
      ctx.beginPath();
      ctx.moveTo(shape.x1, shape.y1);
      ctx.lineTo(shape.x2, shape.y2);
      ctx.stroke();

      // Arrow head calculations
      const angle = Math.atan2(shape.y2 - shape.y1, shape.x2 - shape.x1);
      const headLength = 15;
      ctx.beginPath();
      ctx.moveTo(shape.x2, shape.y2);
      ctx.lineTo(
        shape.x2 - headLength * Math.cos(angle - Math.PI / 6),
        shape.y2 - headLength * Math.sin(angle - Math.PI / 6)
      );
      ctx.lineTo(
        shape.x2 - headLength * Math.cos(angle + Math.PI / 6),
        shape.y2 - headLength * Math.sin(angle + Math.PI / 6)
      );
      ctx.closePath();
      ctx.fillStyle = shape.color;
      ctx.fill();
    }
  }

  function drawMagnifier(ctx: CanvasRenderingContext2D) {
    if (!screenshotElement) return;
    
    const r = magnifierSize / 2;

    // Draw Loupe Outer Glass Border
    ctx.save();
    ctx.shadowColor = "rgba(0, 0, 0, 0.4)";
    ctx.shadowBlur = 15;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 8;

    ctx.beginPath();
    ctx.arc(mouseX, mouseY, r, 0, Math.PI * 2);
    ctx.fillStyle = "#1c1c1e";
    ctx.fill();
    ctx.restore();

    // Clip the zoomed view inside the circle
    ctx.save();
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, r - 3, 0, Math.PI * 2);
    ctx.clip();

    // Calculate source coords on the original screenshot
    const scaleX = screenshotElement.naturalWidth / window.innerWidth;
    const scaleY = screenshotElement.naturalHeight / window.innerHeight;
    
    const sourceWidth = (magnifierSize / zoomLevel) * scaleX;
    const sourceHeight = (magnifierSize / zoomLevel) * scaleY;
    const sourceX = (mouseX * scaleX) - (sourceWidth / 2);
    const sourceY = (mouseY * scaleY) - (sourceHeight / 2);

    // Draw zoomed-in image portion
    ctx.drawImage(
      screenshotElement,
      sourceX, sourceY, sourceWidth, sourceHeight,
      mouseX - r, mouseY - r, magnifierSize, magnifierSize
    );

    // Draw overlay drawings inside the magnifier if any
    if (showAnnotations) {
      ctx.save();
      // Apply the scale and translation to render shapes zoomed in
      ctx.translate(mouseX - r, mouseY - r);
      ctx.scale(zoomLevel, zoomLevel);
      ctx.translate(-sx, -sy);
      annotations.forEach(shape => drawShape(ctx, shape));
      ctx.restore();
    }

    ctx.restore();

    // Draw circular frame border
    ctx.strokeStyle = "rgba(255, 255, 255, 0.8)";
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, r, 0, Math.PI * 2);
    ctx.stroke();

    // Draw target crosshair in the center of the loupe
    ctx.strokeStyle = "rgba(255, 255, 255, 0.5)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(mouseX - 10, mouseY);
    ctx.lineTo(mouseX + 10, mouseY);
    ctx.moveTo(mouseX, mouseY - 10);
    ctx.lineTo(mouseX, mouseY + 10);
    ctx.stroke();
  }

  function drawBigCursor(ctx: CanvasRenderingContext2D) {
    const size = 50; // Cursor halo size
    ctx.save();
    // Semi-transparent colored spotlight halo
    ctx.fillStyle = `${activeColor}33`; // 20% opacity
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, size, 0, Math.PI * 2);
    ctx.fill();

    // Border
    ctx.strokeStyle = activeColor;
    ctx.lineWidth = 1.5;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, size, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }

  // Mouse Handlers for Drawing
  function handleMouseDown(e: MouseEvent) {
    if (activeTool === "none" || activeTool === "magnifier" || activeTool === "cursor") return;
    
    isDrawing = true;
    const rect = mainCanvas?.getBoundingClientRect();
    const x = e.clientX - (rect?.left || 0);
    const y = e.clientY - (rect?.top || 0);

    tempShape = {
      type: activeTool as "box" | "circle" | "arrow",
      x1: x,
      y1: y,
      x2: x,
      y2: y,
      color: activeColor
    };
    
    syncWithMirror();
  }

  function handleMouseMove(e: MouseEvent) {
    const rect = mainCanvas?.getBoundingClientRect();
    mouseX = e.clientX - (rect?.left || 0);
    mouseY = e.clientY - (rect?.top || 0);
    isMouseInCanvas = true;

    if (isDrawing && tempShape) {
      tempShape.x2 = mouseX;
      tempShape.y2 = mouseY;
    }

    syncWithMirror();
  }

  function handleMouseUp() {
    if (isDrawing && tempShape) {
      // Minimum drag check to avoid empty shapes on quick clicks
      const dist = Math.hypot(tempShape.x2 - tempShape.x1, tempShape.y2 - tempShape.y1);
      if (dist > 5) {
        annotations.push(tempShape);
      }
      tempShape = null;
      isDrawing = false;
      syncWithMirror();
    }
  }

  function handleMouseEnter() {
    isMouseInCanvas = true;
    syncWithMirror();
  }

  function handleMouseLeave() {
    isMouseInCanvas = false;
    isDrawing = false;
    tempShape = null;
    syncWithMirror();
  }

  function handleWheel(e: WheelEvent) {
    if (activeTool !== "magnifier") return;
    // Prevent default scrolling behavior
    e.preventDefault();
    if (e.ctrlKey) {
      // Ctrl + Scroll: Resize magnifier circle
      magnifierSize = Math.max(120, Math.min(600, magnifierSize + (e.deltaY < 0 ? 20 : -20)));
    } else {
      // Scroll alone: Adjust zoom magnification
      zoomLevel = Math.max(1.5, Math.min(8, zoomLevel + (e.deltaY < 0 ? 0.2 : -0.2)));
    }
    syncWithMirror();
  }
</script>

<main class="canvas-viewport">
  {#if screenshotUrl}
    <!-- Hidden image used as source for canvas drawImage -->
    <img 
      src={screenshotUrl} 
      alt="Screen Capture" 
      class="hidden-source" 
      bind:this={screenshotElement}
    />
  {/if}

  <canvas
    bind:this={mainCanvas}
    onmousedown={handleMouseDown}
    onmousemove={handleMouseMove}
    onmouseup={handleMouseUp}
    onmouseenter={handleMouseEnter}
    onmouseleave={handleMouseLeave}
    onwheel={handleWheel}
    class="interactive-canvas"
    class:crosshair={activeTool !== "none" && activeTool !== "cursor"}
    class:no-cursor={activeTool === "magnifier" || activeTool === "cursor"}
  ></canvas>

  <!-- Escape to exit tip overlay -->
  <div class="keyboard-tip">
    Press <span>ESC</span> to exit review mode • Hold <span>Space</span> to hide annotations • Use <span>[</span> / <span>]</span> or <span>Scroll</span> to zoom • <span>{`{`}</span> / <span>{`}`}</span> or <span>Ctrl+Scroll</span> to resize magnifier
  </div>
</main>

<style>
  .canvas-viewport {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: transparent;
  }

  .hidden-source {
    display: none;
  }

  .interactive-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }

  .interactive-canvas.crosshair {
    cursor: crosshair;
  }

  .interactive-canvas.no-cursor {
    cursor: none;
  }

  .keyboard-tip {
    position: absolute;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(28, 28, 30, 0.8);
    backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.85);
    padding: 8px 18px;
    border-radius: 20px;
    font-size: 13px;
    pointer-events: none;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    font-weight: 500;
  }

  .keyboard-tip span {
    background: rgba(255, 255, 255, 0.2);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-weight: bold;
    color: #ffffff;
    font-size: 11px;
  }
</style>
