import { writable } from 'svelte/store';

// Define the shape of our settings
export interface AiSettings {
  provider: string;
  model: string;
  apiKeys: Record<string, string>;
  history: Array<{ timestamp: string; imageUrl: string; result: string; model?: string }>;
  totalCount: number;
}

const defaultSettings: AiSettings = {
  provider: "OpenRouter",
  model: "google/gemini-flash-1.5",
  apiKeys: {},
  history: [],
  totalCount: 0
};

// Create a custom store that syncs with localStorage
function createSettingsStore() {
  const isBrowser = typeof window !== 'undefined';
  
  // Load initial value from localStorage if available
  const initialValue = isBrowser 
    ? JSON.parse(localStorage.getItem('ai_settings') || JSON.stringify(defaultSettings))
    : defaultSettings;

  // Migration for old settings without totalCount
  if (initialValue && initialValue.totalCount === undefined) {
    initialValue.totalCount = 0;
  }

  const { subscribe, set, update } = writable<AiSettings>(initialValue);

  return {
    subscribe,
    set: (value: AiSettings) => {
      if (isBrowser) localStorage.setItem('ai_settings', JSON.stringify(value));
      set(value);
    },
    update: (updater: (value: AiSettings) => AiSettings) => {
      update(currentValue => {
        const newValue = updater(currentValue);
        if (isBrowser) localStorage.setItem('ai_settings', JSON.stringify(newValue));
        return newValue;
      });
    },
    // Helper to reload store from localStorage (for syncing across windows)
    load: () => {
      if (isBrowser) {
        const data = localStorage.getItem('ai_settings');
        if (data) {
          const parsed = JSON.parse(data);
          if (parsed.totalCount === undefined) parsed.totalCount = 0;
          set(parsed);
        }
      }
    },
    // Helper to add history
    addHistory: (imageUrl: string, result: string) => {
      update(settings => {
        const newHistory = [
          { 
            timestamp: new Date().toISOString(), 
            imageUrl, 
            result, 
            model: settings.model 
          }, 
          ...settings.history
        ];
        const newCount = (settings.totalCount || 0) + 1;
        const newState = { ...settings, history: newHistory, totalCount: newCount };
        if (isBrowser) localStorage.setItem('ai_settings', JSON.stringify(newState));
        return newState;
      });
    },
    clearHistory: () => {
      update(settings => {
        const newState = { ...settings, history: [] };
        if (isBrowser) localStorage.setItem('ai_settings', JSON.stringify(newState));
        return newState;
      });
    },
    clearCount: () => {
      update(settings => {
        const newState = { ...settings, totalCount: 0 };
        if (isBrowser) localStorage.setItem('ai_settings', JSON.stringify(newState));
        return newState;
      });
    }
  };
}

export const settingsStore = createSettingsStore();
