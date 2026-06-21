export interface AppSettings {
  propresenter_host: string;
  propresenter_port: number;
  propresenter_endpoint: string;
  worker_base_url: string;
  admin_token: string;
  worker_slug: string;
  minimum_kv_write_spacing_ms: number;
  default_redirect_url: string;
  link_owner_id: string;
  tag_mappings: Record<string, string>;
}

export interface WorkerConfig {
  base_url: string;
  admin_token: string;
  slug: string;
}

export interface TimedTag {
  seconds: number;
  keyword: string;
  raw_time: string;
}

export interface ParsedNotes {
  regular_tags: string[];
  timed_tags: TimedTag[];
  invalid_tags: string[];
}

export interface SlideUpdate {
  generation: number;
  notes: string;
  parsed: ParsedNotes;
  media_started: boolean;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  type: "info" | "success" | "warning" | "error" | "event";
  message: string;
  tag?: string;
}
