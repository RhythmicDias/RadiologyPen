import { get } from 'svelte/store';
import { settingsStore } from './settingsStore';

export async function analyzeImage(base64Image: string, onChunk: (text: string) => void): Promise<string> {
  settingsStore.load();
  const settings = get(settingsStore);
  const apiKey = settings.apiKeys["OpenRouter"];
  
  if (!apiKey) {
    throw new Error(`No API key set for OpenRouter. Please configure it in settings.`);
  }

  try {
    return await callOpenRouterStream(apiKey, settings.model || "google/gemini-flash-1.5", base64Image, onChunk);
  } catch (error: any) {
    console.error("AI Analysis Error:", error);
    throw new Error(error.message || "Failed to analyze image");
  }
}

async function callOpenRouterStream(
  apiKey: string, 
  model: string, 
  base64Image: string, 
  onChunk: (text: string) => void
): Promise<string> {
  const res = await fetch("https://openrouter.ai/api/v1/chat/completions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": `Bearer ${apiKey}`
    },
    body: JSON.stringify({
      model: model,
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "Perform an expert radiologic and neurophysiologic review of this medical/clinical image crop. Provide a concise description of the observed structures, features, waveforms, or potential anomalies, followed by a provisional diagnosis if applicable. Do not include any clinical disclaimers, caveats, final notes, or templates stating that this is not a definitive diagnosis—write only the clinical review itself." },
            { type: "image_url", image_url: { url: base64Image } }
          ]
        }
      ],
      stream: true
    })
  });
  
  if (!res.ok) {
    const errText = await res.text();
    let errMsg = res.statusText;
    try {
      const errJson = JSON.parse(errText);
      if (errJson.error) errMsg = errJson.error.message;
    } catch {}
    throw new Error(`OpenRouter Error: ${errMsg}`);
  }

  const reader = res.body?.getReader();
  if (!reader) {
    throw new Error("Response body is not readable");
  }

  const decoder = new TextDecoder("utf-8");
  let fullText = "";
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    
    buffer = lines.pop() || "";
    
    for (const line of lines) {
      const cleaned = line.trim();
      if (!cleaned || cleaned === "data: [DONE]") continue;
      
      if (cleaned.startsWith("data: ")) {
        const jsonStr = cleaned.slice(6);
        try {
          const parsed = JSON.parse(jsonStr);
          const chunk = parsed.choices?.[0]?.delta?.content || "";
          if (chunk) {
            fullText += chunk;
            onChunk(fullText);
          }
        } catch (e) {
          // Ignore parsing errors for partial or meta lines
        }
      }
    }
  }

  // Handle final buffer chunk
  if (buffer.startsWith("data: ")) {
    try {
      const parsed = JSON.parse(buffer.slice(6));
      const chunk = parsed.choices?.[0]?.delta?.content || "";
      if (chunk) {
        fullText += chunk;
        onChunk(fullText);
      }
    } catch {}
  }

  if (!fullText) {
    throw new Error("OpenRouter returned an empty stream response.");
  }

  return fullText;
}
