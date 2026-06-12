import { invoke } from "@tauri-apps/api/core";

export interface DayStats {
  active_seconds: number;
  session_count: number;
  char_count: number;
  pending_chars: number;
}

export interface AppStatus {
  recording: boolean;
  consent: boolean;
  has_api_key: boolean;
  day: string;
  stats: DayStats;
}

export interface Summary {
  day: string;
  content: string;
  created_at: string;
}

export interface Fact {
  id?: number;
  subject: string;
  predicate: string;
  object: string;
  category: string;
  confidence: number;
  observations: number;
  valid_at: string;
  invalid_at?: string | null;
  source_day: string;
  created_at: string;
}

export interface HabitProfile {
  window_days: number;
  peak_period: string;
  avg_session_minutes: number;
  switch_frequency: number;
  top_apps: [string, number][];
  top_projects: [string, number][];
}

export function factStatement(f: Fact): string {
  return `${f.subject} ${f.predicate} ${f.object}`;
}

export const api = {
  getStatus: () => invoke<AppStatus>("get_status"),
  setRecording: (recording: boolean) => invoke<void>("set_recording", { recording }),
  setConsent: (accepted: boolean) => invoke<void>("set_consent", { accepted }),
  setApiKey: (key: string) => invoke<void>("set_api_key", { key }),
  generateSummary: (day?: string) => invoke<Summary>("generate_summary", { day }),
  getSummary: (day?: string) => invoke<Summary | null>("get_summary", { day }),
  clearAllData: () => invoke<void>("clear_all_data"),
  listFacts: () => invoke<Fact[]>("list_facts"),
  deleteFact: (id: number) => invoke<void>("delete_fact", { id }),
  extractFacts: (day?: string) => invoke<number>("extract_facts", { day }),
  consolidateFacts: (day?: string) => invoke<Fact[]>("consolidate_facts", { day }),
  getHabitProfile: () => invoke<HabitProfile>("get_habit_profile"),
  exportHermesMemory: () => invoke<string>("export_hermes_memory"),
  getHermesExportDir: () => invoke<string>("get_hermes_export_dir"),
  setHermesExportDir: (path: string) => invoke<void>("set_hermes_export_dir", { path }),
  getAutoExport: () => invoke<boolean>("get_auto_export"),
  setAutoExport: (enabled: boolean) => invoke<void>("set_auto_export", { enabled }),
};
