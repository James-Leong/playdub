export interface PlaydubSettings {
  sourceLanguage: string;
  targetLanguage: string;
  showSourceCaptions: boolean;
}

export const DEFAULT_SETTINGS: PlaydubSettings = {
  sourceLanguage: "auto",
  targetLanguage: "zh",
  showSourceCaptions: true,
};

const SETTINGS_KEY = "playdub.settings.v1";

export function loadSettings(): PlaydubSettings {
  try {
    const stored = window.localStorage.getItem(SETTINGS_KEY);
    if (!stored) {
      return DEFAULT_SETTINGS;
    }
    const parsed = JSON.parse(stored) as Partial<PlaydubSettings>;
    return {
      sourceLanguage:
        typeof parsed.sourceLanguage === "string"
          ? parsed.sourceLanguage
          : DEFAULT_SETTINGS.sourceLanguage,
      targetLanguage:
        typeof parsed.targetLanguage === "string"
          ? parsed.targetLanguage
          : DEFAULT_SETTINGS.targetLanguage,
      showSourceCaptions:
        typeof parsed.showSourceCaptions === "boolean"
          ? parsed.showSourceCaptions
          : DEFAULT_SETTINGS.showSourceCaptions,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: PlaydubSettings): void {
  window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}
