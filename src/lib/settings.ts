export interface PlaydubSettings {
  ffmpegPath: string;
  whisperCliPath: string;
  whisperModelPath: string;
  llamaCliPath: string;
  llamaModelPath: string;
  sourceLanguage: string;
  targetLanguage: string;
  showSourceCaptions: boolean;
}

export const DEFAULT_SETTINGS: PlaydubSettings = {
  ffmpegPath: "",
  whisperCliPath: "",
  whisperModelPath: "",
  llamaCliPath: "",
  llamaModelPath: "",
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
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: PlaydubSettings): void {
  window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}
