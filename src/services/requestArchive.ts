import { invoke } from '@tauri-apps/api/core';

export type RequestArchiveSettings = {
  enabled: boolean;
  retentionDays: number;
  maxTotalMb: number;
  maxBodyKb: number;
};

export type RequestArchiveStatus = RequestArchiveSettings & {
  settingsEnabled: boolean;
  requestLogEnabled: boolean;
  recordCount: number;
  databaseBytes: number;
  databasePath: string;
  logsDirectory: string;
  lastIngestedAt: string;
  lastError: string;
  oldestRecordAt: string;
  newestRecordAt: string;
};

export type RequestArchiveSummary = {
  id: number;
  requestId: string;
  capturedAt: string;
  capturedAtMs: number;
  url: string;
  endpoint: string;
  method: string;
  model: string;
  stream: boolean;
  httpStatus: number;
  apiErrorStatus: number;
  failed: boolean;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  hasSystemPrompt: boolean;
  hasTools: boolean;
  truncated: boolean;
  byteSize: number;
  sourceFile: string;
};

export type RequestArchiveDetail = RequestArchiveSummary & {
  coreVersion: string;
  downstreamTransport: string;
  upstreamTransport: string;
  systemPrompt: string;
  messagesJson: string;
  toolsJson: string;
  usageJson: string;
  reasoningTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  requestHeadersJson: string;
  responseHeadersJson: string;
  requestBody: string;
  apiRequest: string;
  apiResponse: string;
  responseBody: string;
  apiErrorText: string;
  websocketTimeline: string;
};

export type RequestArchiveQuery = {
  startMs?: number;
  endMs?: number;
  model?: string;
  result?: 'all' | 'success' | 'failed';
  search?: string;
  requestId?: string;
  limit?: number;
  offset?: number;
};

export type RequestArchivePage = {
  total: number;
  records: RequestArchiveSummary[];
};

export const requestArchiveApi = {
  status: () => invoke<RequestArchiveStatus>('get_request_archive_status'),
  saveSettings: (settings: RequestArchiveSettings) =>
    invoke<void>('save_request_archive_settings', { settings }),
  query: (query: RequestArchiveQuery) =>
    invoke<RequestArchivePage>('query_request_archive_records', { query }),
  record: (id: number) => invoke<RequestArchiveDetail | null>('get_request_archive_record', { id }),
  recordByRequestId: (requestId: string) =>
    invoke<RequestArchiveDetail | null>('get_request_archive_record_by_request_id', { requestId }),
  models: () => invoke<string[]>('get_request_archive_models'),
  clear: () => invoke<void>('clear_request_archive'),
};

export type ArchiveReadiness = 'disabled' | 'awaiting-request-log' | 'ready';

/**
 * Archiving only produces rows when the core is also writing request logs, so a
 * enabled archive without `request-log` is reported as an actionable state.
 */
export function archiveReadiness(status: Pick<RequestArchiveStatus, 'settingsEnabled' | 'requestLogEnabled'>): ArchiveReadiness {
  if (!status.settingsEnabled) {
    return 'disabled';
  }
  return status.requestLogEnabled ? 'ready' : 'awaiting-request-log';
}

export function formatArchiveBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const rendered = unit === 0 ? String(Math.round(value)) : value.toFixed(value >= 100 ? 0 : 1);
  return `${rendered} ${units[unit]}`;
}

export type ArchiveMessage = {
  role: string;
  text: string;
  toolCalls: string[];
};

function collectText(value: unknown, parts: string[]): void {
  if (typeof value === 'string') {
    if (value.trim()) {
      parts.push(value);
    }
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((entry) => collectText(entry, parts));
    return;
  }
  if (value && typeof value === 'object') {
    const record = value as Record<string, unknown>;
    if (typeof record.text === 'string') {
      if (record.text.trim()) {
        parts.push(record.text);
      }
      return;
    }
    ['parts', 'content'].forEach((key) => {
      if (key in record) {
        collectText(record[key], parts);
      }
    });
  }
}

function collectToolCalls(entry: Record<string, unknown>): string[] {
  const names: string[] = [];
  const push = (name: unknown) => {
    if (typeof name === 'string' && name.trim()) {
      names.push(name);
    }
  };

  // OpenAI chat completions.
  if (Array.isArray(entry.tool_calls)) {
    entry.tool_calls.forEach((call) => {
      const record = call as Record<string, unknown>;
      const fn = record.function as Record<string, unknown> | undefined;
      push(fn?.name ?? record.name);
    });
  }
  // Claude content blocks and OpenAI Responses items.
  const content = entry.content ?? entry.parts;
  if (Array.isArray(content)) {
    content.forEach((block) => {
      const record = block as Record<string, unknown>;
      if (record.type === 'tool_use' || record.type === 'tool_result') {
        push(record.name);
      }
      const functionCall = record.functionCall as Record<string, unknown> | undefined;
      push(functionCall?.name);
    });
  }
  if (entry.type === 'function_call') {
    push(entry.name);
  }
  return names;
}

/**
 * Normalises the stored `messages` / `contents` / `input` array of any supported
 * provider protocol into a flat list for display.
 */
export function parseArchiveMessages(messagesJson: string): ArchiveMessage[] {
  if (!messagesJson.trim()) {
    return [];
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(messagesJson);
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) {
    return [];
  }
  return parsed.map((entry) => {
    if (typeof entry === 'string') {
      return { role: 'user', text: entry, toolCalls: [] };
    }
    const record = (entry ?? {}) as Record<string, unknown>;
    const parts: string[] = [];
    collectText(record.content ?? record.parts ?? record.text ?? null, parts);
    const role =
      (typeof record.role === 'string' && record.role) ||
      (typeof record.type === 'string' && record.type) ||
      'unknown';
    return { role, text: parts.join('\n'), toolCalls: collectToolCalls(record) };
  });
}

/** Extracts the declared tool names from the stored tools payload. */
export function parseArchiveToolNames(toolsJson: string): string[] {
  if (!toolsJson.trim()) {
    return [];
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(toolsJson);
  } catch {
    return [];
  }
  const entries = Array.isArray(parsed)
    ? parsed
    : Array.isArray((parsed as Record<string, unknown>)?.functionDeclarations)
      ? ((parsed as Record<string, unknown>).functionDeclarations as unknown[])
      : [];
  const names: string[] = [];
  entries.forEach((entry) => {
    const record = (entry ?? {}) as Record<string, unknown>;
    const fn = record.function as Record<string, unknown> | undefined;
    const name = record.name ?? fn?.name;
    if (typeof name === 'string' && name.trim()) {
      names.push(name);
    }
    if (Array.isArray(record.functionDeclarations)) {
      record.functionDeclarations.forEach((declaration) => {
        const declared = (declaration as Record<string, unknown>)?.name;
        if (typeof declared === 'string' && declared.trim()) {
          names.push(declared);
        }
      });
    }
  });
  return names;
}

/** Pretty-prints a stored JSON blob, falling back to the raw text. */
export function formatArchiveJson(value: string): string {
  if (!value.trim()) {
    return '';
  }
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}
