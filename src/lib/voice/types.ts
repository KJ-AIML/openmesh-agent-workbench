export type VoicePhase =
  | "off"
  | "idle"
  | "listening"
  | "thinking"
  | "speaking"
  | "error";

export type VoiceUiAction = {
  action: "ui_navigate";
  route: string;
  label?: string;
};

export type VoiceTurnLog = {
  id: string;
  heard: string;
  reply: string;
  actions: VoiceUiAction[];
  at: number;
  error?: string;
};
