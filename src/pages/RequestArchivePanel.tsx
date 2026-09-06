import { useCallback, useEffect, useMemo, useState } from 'react';
import { Archive, FolderOpen, HardDrive, RefreshCw, Trash2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../i18n';
import type { MessageKey } from '../i18n/resources';
import {
  archiveReadiness,
  formatArchiveBytes,
  formatArchiveJson,
  parseArchiveMessages,
  parseArchiveToolNames,
  requestArchiveApi,
  requestArchiveSettingsDraft,
  requestArchiveSettingsFromDraft,
  type RequestArchiveDetail,
  type RequestArchivePayloads,
  type RequestArchiveSettingsDraft,
  type RequestArchiveStatus,
} from '../services/requestArchive';
import '../styles/requestArchive.css';

function useArchiveStatus() {
  const [status, setStatus] = useState<RequestArchiveStatus | null>(null);
  const [error, setError] = useState('');

  const refresh = useCallback(async () => {
    try {
      setStatus(await requestArchiveApi.status());
      setError('');
    } catch (statusError) {
      setError(String(statusError));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { status, error, refresh, setError };
}

/**
 * Reports whether the fork-local request archive is compiled in and not killed by
 * the CPA_FORK_ARCHIVE environment switch. Returns `null` while still resolving so
 * callers can avoid flashing UI that is about to be hidden.
 */
export function useRequestArchiveAvailable(): boolean | null {
  const [available, setAvailable] = useState<boolean | null>(null);

  useEffect(() => {
    let active = true;
    requestArchiveApi
      .status()
      .then((status) => active && setAvailable(!status.forkDisabled))
      .catch(() => active && setAvailable(false));
    return () => {
      active = false;
    };
  }, []);

  return available;
}

export function RequestArchiveSettingsCard() {
  const { t } = useI18n();
  const { status, error, refresh, setError } = useArchiveStatus();
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState('');
  const [draft, setDraft] = useState<RequestArchiveSettingsDraft | null>(null);

  const readiness = status ? archiveReadiness(status) : 'disabled';
  const settings = useMemo(
    () => (draft ? requestArchiveSettingsFromDraft(draft) : null),
    [draft],
  );

  useEffect(() => {
    if (status) {
      setDraft(requestArchiveSettingsDraft(status));
    }
  }, [status]);

  if (status?.forkDisabled) {
    return null;
  }

  const saveSettings = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await requestArchiveApi.saveSettings(settings);
      await refresh();
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setSaving(false);
    }
  };

  const updateDraft = (
    field: Exclude<keyof RequestArchiveSettingsDraft, 'enabled'>,
    value: string,
  ) => {
    setDraft((current) => (current ? { ...current, [field]: value } : current));
  };

  const setRequestLog = async (enabled: boolean) => {
    setSaving(true);
    try {
      await invoke('set_core_request_log', { enabled });
      await refresh();
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setSaving(false);
    }
  };

  const runMaintenance = async (
    action: 'compact' | 'purgeLogs',
    confirmKey?: MessageKey,
  ) => {
    if (confirmKey && !window.confirm(t(confirmKey))) return;
    setSaving(true);
    setNotice('');
    try {
      const result = await requestArchiveApi[action]();
      setNotice(
        t('usage.archive.maintenanceResult', {
          files: String(result.removedFiles),
          freed: formatArchiveBytes(result.freedBytes),
          skipped: String(result.skippedFiles),
        }),
      );
      await refresh();
    } catch (maintenanceError) {
      setError(String(maintenanceError));
    } finally {
      setSaving(false);
    }
  };

  const clear = async () => {
    if (!window.confirm(t('usage.archive.clearConfirm'))) return;
    setSaving(true);
    try {
      await requestArchiveApi.clear();
      await refresh();
    } catch (clearError) {
      setError(String(clearError));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="panel usage-data-management-panel">
      <div className="usage-data-management-heading">
        <div>
          <Archive size={20} aria-hidden="true" />
          <div>
            <h2>{t('usage.archive.title')}</h2>
            <p>{t('usage.archive.description')}</p>
          </div>
        </div>
      </div>

      {error ? <p className="usage-archive-alert">{error}</p> : null}
      {notice ? <p className="usage-archive-notice">{notice}</p> : null}

      {status ? (
        <>
          <div className="usage-archive-switches">
            <label className="usage-archive-switch">
              <input
                type="checkbox"
                checked={status.requestLogEnabled}
                disabled={saving || status.commercialMode}
                onChange={(event) => void setRequestLog(event.target.checked)}
              />
              <span>{t('usage.archive.requestLogSwitch')}</span>
            </label>
          </div>

          {readiness === 'commercial-mode' ? (
            <p className="usage-archive-alert">{t('usage.archive.commercialModeBlocked')}</p>
          ) : null}

          {readiness === 'awaiting-request-log' ? (
            <p className="usage-archive-alert">{t('usage.archive.requestLogRequired')}</p>
          ) : null}

          <form
            onSubmit={(event) => {
              event.preventDefault();
              void saveSettings();
            }}
          >
            <div className="usage-archive-switches">
              <label className="usage-archive-switch">
                <input
                  type="checkbox"
                  checked={draft?.enabled ?? status.settingsEnabled}
                  disabled={saving}
                  onChange={(event) =>
                    setDraft((current) =>
                      current ? { ...current, enabled: event.target.checked } : current,
                    )
                  }
                />
                <span>{t('usage.archive.enable')}</span>
              </label>
            </div>

            <div className="usage-archive-fields">
              <label>
                <span>{t('usage.archive.retentionDays')}</span>
                <input
                  type="number"
                  min={0}
                  max={3650}
                  value={draft?.retentionDays ?? String(status.retentionDays)}
                  disabled={saving}
                  onChange={(event) => updateDraft('retentionDays', event.target.value)}
                />
              </label>
              <label>
                <span>{t('usage.archive.maxTotalMb')}</span>
                <input
                  type="number"
                  min={0}
                  max={1048576}
                  value={draft?.maxTotalMb ?? String(status.maxTotalMb)}
                  disabled={saving}
                  onChange={(event) => updateDraft('maxTotalMb', event.target.value)}
                />
              </label>
              <label>
                <span>{t('usage.archive.logsMaxMb')}</span>
                <input
                  type="number"
                  min={0}
                  max={1048576}
                  value={draft?.logsMaxMb ?? String(status.logsMaxMb)}
                  disabled={saving}
                  onChange={(event) => updateDraft('logsMaxMb', event.target.value)}
                />
              </label>
              <label>
                <span>{t('usage.archive.maxBodyKb')}</span>
                <input
                  type="number"
                  min={16}
                  max={65536}
                  value={draft?.maxBodyKb ?? String(status.maxBodyKb)}
                  disabled={saving}
                  onChange={(event) => updateDraft('maxBodyKb', event.target.value)}
                />
              </label>
            </div>

            <div className="usage-archive-actions">
              <button type="submit" disabled={saving || !settings}>
                <span>{t('usage.archive.saveSettings')}</span>
              </button>
            </div>
          </form>

          <dl className="usage-archive-stats">
            <div>
              <dt>{t('usage.archive.recordCount')}</dt>
              <dd>{status.recordCount}</dd>
            </div>
            <div>
              <dt>{t('usage.archive.databaseSize')}</dt>
              <dd>{formatArchiveBytes(status.databaseBytes)}</dd>
            </div>
            <div>
              <dt>{t('usage.archive.logsSize')}</dt>
              <dd>{formatArchiveBytes(status.logsBytes)}</dd>
            </div>
            <div>
              <dt>{t('usage.archive.lastIngestedAt')}</dt>
              <dd>{status.lastIngestedAt || '—'}</dd>
            </div>
          </dl>

          <div className="usage-archive-paths">
            <p title={status.databasePath}>
              <span>{t('usage.archive.databasePath')}</span>
              <code>{status.databasePath}</code>
            </p>
            <p title={status.logsDirectory}>
              <span>{t('usage.archive.logsDirectory')}</span>
              <code>{status.logsDirectory}</code>
            </p>
          </div>

          {status.lastError ? <p className="usage-archive-alert">{status.lastError}</p> : null}

          <div className="usage-archive-actions">
            <button type="button" disabled={saving} onClick={() => void refresh()}>
              <RefreshCw size={16} aria-hidden="true" />
              <span>{t('usage.archive.refresh')}</span>
            </button>
            <button type="button" disabled={saving} onClick={() => void invoke('open_core_logs_directory')}>
              <FolderOpen size={16} aria-hidden="true" />
              <span>{t('usage.archive.openLogs')}</span>
            </button>
            <button type="button" disabled={saving} onClick={() => void runMaintenance('compact')}>
              <HardDrive size={16} aria-hidden="true" />
              <span>{t('usage.archive.compact')}</span>
            </button>
            <button
              type="button"
              disabled={saving}
              onClick={() => void runMaintenance('purgeLogs', 'usage.archive.purgeLogsConfirm')}
            >
              <Trash2 size={16} aria-hidden="true" />
              <span>{t('usage.archive.purgeLogs')}</span>
            </button>
            <button
              type="button"
              className="usage-archive-danger"
              disabled={saving}
              onClick={() => void clear()}
            >
              {t('usage.archive.clear')}
            </button>
          </div>
        </>
      ) : null}
    </section>
  );
}

type DetailProps = {
  requestId: string;
  onClose: () => void;
};

type DetailSection = 'system' | 'messages' | 'tools' | 'usage' | 'raw';

const SECTION_LABELS = {
  messages: 'usage.archive.section.messages',
  system: 'usage.archive.section.system',
  tools: 'usage.archive.section.tools',
  usage: 'usage.archive.section.usage',
  raw: 'usage.archive.section.raw',
} as const;

const SECTION_ORDER: DetailSection[] = ['messages', 'system', 'tools', 'usage', 'raw'];

export function RequestArchiveDetailDialog({ requestId, onClose }: DetailProps) {
  const { t } = useI18n();
  const available = useRequestArchiveAvailable();
  const [detail, setDetail] = useState<RequestArchiveDetail | null>(null);
  const [payloads, setPayloads] = useState<RequestArchivePayloads | null>(null);
  const [payloadsLoading, setPayloadsLoading] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [section, setSection] = useState<DetailSection>('messages');

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError('');
    requestArchiveApi
      .recordByRequestId(requestId)
      .then((record) => {
        if (!active) return;
        setDetail(record);
      })
      .catch((detailError) => {
        if (!active) return;
        setError(String(detailError));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [requestId]);

  const messages = useMemo(
    () => (detail ? parseArchiveMessages(detail.messagesJson) : []),
    [detail],
  );
  const toolNames = useMemo(() => (detail ? parseArchiveToolNames(detail.toolsJson) : []), [detail]);

  // Raw payloads are several megabytes per record, so they are only fetched once
  // the reader actually opens the raw tab, and only once per record.
  useEffect(() => {
    if (section !== 'raw' || !detail || payloads || payloadsLoading) {
      return;
    }
    let active = true;
    setPayloadsLoading(true);
    requestArchiveApi
      .payloads(detail.id)
      .then((result) => active && setPayloads(result))
      .catch((payloadError) => active && setError(String(payloadError)))
      .finally(() => active && setPayloadsLoading(false));
    return () => {
      active = false;
    };
  }, [section, detail, payloads, payloadsLoading]);

  useEffect(() => {
    setPayloads(null);
  }, [requestId]);

  if (available === false) {
    return null;
  }

  return (
    <div
      className="config-dialog-backdrop"
      onMouseDown={(event) => event.currentTarget === event.target && onClose()}
    >
      <section
        className="config-dialog usage-archive-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t('usage.archive.detailTitle')}
        onKeyDown={(event) => event.key === 'Escape' && onClose()}
        tabIndex={-1}
      >
        <header className="usage-archive-dialog-header">
          <h2>{t('usage.archive.detailTitle')}</h2>
          <button
            type="button"
            className="usage-archive-close"
            aria-label={t('usage.archive.close')}
            onClick={onClose}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        {detail ? (
          <>
            <p className="usage-archive-meta">
              <span>{detail.method}</span>
              <span>{detail.endpoint}</span>
              <span>{detail.model || '—'}</span>
              <span>HTTP {detail.httpStatus}</span>
              <span>{detail.capturedAt}</span>
              {detail.truncated ? (
                <span className="usage-archive-truncated">{t('usage.archive.truncated')}</span>
              ) : null}
            </p>

            <nav className="usage-archive-tabs">
              {SECTION_ORDER.map((key) => (
                <button
                  key={key}
                  type="button"
                  className={section === key ? 'active' : ''}
                  onClick={() => setSection(key)}
                >
                  {t(SECTION_LABELS[key])}
                </button>
              ))}
            </nav>
          </>
        ) : null}

        <div className="usage-archive-dialog-body">
          {loading ? <p className="usage-archive-placeholder">{t('usage.archive.loading')}</p> : null}
          {error ? <p className="usage-archive-alert">{error}</p> : null}
          {!loading && !error && !detail ? (
            <p className="usage-archive-placeholder">{t('usage.archive.notFound')}</p>
          ) : null}

          {detail && section === 'system' ? (
            <pre className="usage-archive-content">{detail.systemPrompt || t('usage.archive.empty')}</pre>
          ) : null}

          {detail && section === 'messages' ? (
            messages.length === 0 ? (
              <p className="usage-archive-placeholder">{t('usage.archive.empty')}</p>
            ) : (
              <ol className="usage-archive-messages">
                {messages.map((message, index) => (
                  <li key={`${message.role}-${index}`}>
                    <div className="usage-archive-message-head">
                      <span className="usage-archive-role">{message.role}</span>
                      {message.toolCalls.length > 0 ? (
                        <span className="usage-archive-tools">{message.toolCalls.join(', ')}</span>
                      ) : null}
                    </div>
                    <pre className="usage-archive-content">{message.text || t('usage.archive.empty')}</pre>
                  </li>
                ))}
              </ol>
            )
          ) : null}

          {detail && section === 'tools' ? (
            toolNames.length === 0 ? (
              <p className="usage-archive-placeholder">{t('usage.archive.empty')}</p>
            ) : (
              <>
                <p className="usage-archive-toolnames">{toolNames.join(', ')}</p>
                <pre className="usage-archive-content">{formatArchiveJson(detail.toolsJson)}</pre>
              </>
            )
          ) : null}

          {detail && section === 'usage' ? (
            <pre className="usage-archive-content">
              {formatArchiveJson(detail.usageJson) || t('usage.archive.empty')}
            </pre>
          ) : null}

          {detail && section === 'raw' ? (
            payloadsLoading || !payloads ? (
              <p className="usage-archive-placeholder">{t('usage.archive.loading')}</p>
            ) : (
              <>
                <h3 className="usage-archive-subhead">{t('usage.archive.rawRequest')}</h3>
                <pre className="usage-archive-content">
                  {formatArchiveJson(payloads.apiRequest || payloads.requestBody) ||
                    t('usage.archive.empty')}
                </pre>
                <h3 className="usage-archive-subhead">{t('usage.archive.rawResponse')}</h3>
                <pre className="usage-archive-content">
                  {payloads.apiResponse ||
                    payloads.responseBody ||
                    payloads.apiErrorText ||
                    t('usage.archive.empty')}
                </pre>
              </>
            )
          ) : null}
        </div>
      </section>
    </div>
  );
}
