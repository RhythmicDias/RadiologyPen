<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
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

  // Sync state
  let annotations = $state<Shape[]>([]);
  let tempShape = $state<Shape | null>(null);
  let showAnnotations = $state(true);
  let activeTool = $state("none");
  let activeColor = $state("#ff3b30");
  let mouseX = $state(0);
  let mouseY = $state(0);
  let isMouseInCanvas = $state(false);
  let zoomLevel = $state(3);
  let magnifierSize = $state(350);

  let mirrorCanvas = $state<HTMLCanvasElement | null>(null);

  onMount(() => {
    // Listen for background screenshot changes
    const unlistenScreenshot = listen<{ bytes: number[] }>("screenshot-captured", (event) => {
      const bytes = event.payload.bytes;
      const blob = new Blob([new Uint8Array(bytes)], { type: "image/png" });
      if (screenshotUrl) {
        URL.revokeObjectURL(screenshotUrl);
      }
      screenshotUrl = URL.createObjectURL(blob);
    });

    // Listen for sync updates from primary clinician canvas
    const unlistenSync = listen<any>("mirror-sync", (event) => {
      const payload = event.payload;
      annotations = payload.annotations;
      tempShape = payload.tempShape;
      showAnnotations = payload.showAnnotations;
      activeTool = payload.activeTool;
      activeColor = payload.activeColor;
      mouseX = payload.mouseX;
      mouseY = payload.mouseY;
      isMouseInCanvas = payload.isMouseInCanvas;
      zoomLevel = payload.zoomLevel;
      magnifierSize = payload.magnifierSize;
    });

    // Close screen or clear when canvas hides
    const unlistenClose = listen("canvas-closed", () => {
      // Clear mirror view
      screenshotUrl = null;
    });

    return () => {
      unlistenScreenshot.then(f => f());
      unlistenSync.then(f => f());
      unlistenClose.then(f => f());
    };
  });

  // Draw loop
  $effect(() => {
    if (!mirrorCanvas) return;
    const ctx = mirrorCanvas.getContext("2d");
    if (!ctx) return;

    // Set canvas dimensions
    mirrorCanvas.width = window.innerWidth;
    mirrorCanvas.height = window.innerHeight;

    ctx.clearRect(0, 0, mirrorCanvas.width, mirrorCanvas.height);

    if (screenshotElement && screenshotUrl) {
      // Draw background screenshot
      ctx.drawImage(screenshotElement, 0, 0, mirrorCanvas.width, mirrorCanvas.height);

      // Draw saved annotations
      if (showAnnotations) {
        annotations.forEach(shape => drawShape(ctx, shape));
      }

      // Draw temp drawing shape
      if (tempShape) {
        drawShape(ctx, tempShape);
      }

      // Draw Magnifier Zoom
      if (activeTool === "magnifier" && isMouseInCanvas) {
        drawMagnifier(ctx);
      }

      // Draw Big Cursor
      if (activeTool === "cursor" && isMouseInCanvas) {
        drawBigCursor(ctx);
      }
    } else {
      // Black screensaver if idle
      ctx.fillStyle = "#0f0f11";
      ctx.fillRect(0, 0, mirrorCanvas.width, mirrorCanvas.height);

      ctx.fillStyle = "rgba(255, 255, 255, 0.4)";
      ctx.font = "24px 'Inter', sans-serif";
      ctx.textAlign = "center";
      ctx.fillText("EEG Pen — Parent Facing Monitor", mirrorCanvas.width / 2, mirrorCanvas.height / 2);
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
      ctx.beginPath();
      ctx.moveTo(shape.x1, shape.y1);
      ctx.lineTo(shape.x2, shape.y2);
      ctx.stroke();

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

    ctx.save();
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, r - 3, 0, Math.PI * 2);
    ctx.clip();

    const sw = magnifierSize / zoomLevel;
    const sh = magnifierSize / zoomLevel;
    const sx = mouseX - sw / 2;
    const sy = mouseY - sh / 2;

    ctx.drawImage(
      screenshotElement,
      sx, sy, sw, sh,
      mouseX - r, mouseY - r, magnifierSize, magnifierSize
    );

    if (showAnnotations) {
      ctx.save();
      ctx.translate(mouseX - r, mouseY - r);
      ctx.scale(zoomLevel, zoomLevel);
      ctx.translate(-sx, -sy);
      annotations.forEach(shape => drawShape(ctx, shape));
      ctx.restore();
    }
    ctx.restore();

    ctx.strokeStyle = "rgba(255, 255, 255, 0.8)";
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, r, 0, Math.PI * 2);
    ctx.stroke();

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
    const size = 50;
    ctx.save();
    ctx.fillStyle = `${activeColor}33`;
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, size, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = activeColor;
    ctx.lineWidth = 1.5;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.arc(mouseX, mouseY, size, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }
</script>

<main class="mirror-viewport">
  {#if screenshotUrl}
    <img 
      src={screenshotUrl} 
      alt="Screen Capture" 
      class="hidden-source" 
      bind:this={screenshotElement}
    />
  {/if}

  <canvas bind:this={mirrorCanvas} class="output-canvas"></canvas>
</main>

<style>
  .mirror-viewport {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: #0f0f11;
  }

  .hidden-source {
    display: none;
  }

  .output-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
