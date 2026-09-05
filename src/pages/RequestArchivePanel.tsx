import { useCallback, useEffect, useMemo, useState } from 'react';
import { Archive, FolderOpen, RefreshCw, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../i18n';
import {
  archiveReadiness,
  formatArchiveBytes,
  formatArchiveJson,
  parseArchiveMessages,
  parseArchiveToolNames,
  requestArchiveApi,
  type RequestArchiveDetail,
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

export function RequestArchiveSettingsCard() {
  const { t } = useI18n();
  const { status, error, refresh, setError } = useArchiveStatus();
  const [saving, setSaving] = useState(false);

  const readiness = status ? archiveReadiness(status) : 'disabled';

  const persist = async (changes: Partial<RequestArchiveStatus>) => {
    if (!status) return;
    setSaving(true);
    try {
      await requestArchiveApi.saveSettings({
        enabled: changes.settingsEnabled ?? status.settingsEnabled,
        retentionDays: changes.retentionDays ?? status.retentionDays,
        maxTotalMb: changes.maxTotalMb ?? status.maxTotalMb,
        maxBodyKb: changes.maxBodyKb ?? status.maxBodyKb,
      });
      await refresh();
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setSaving(false);
    }
  };

  const enableRequestLog = async () => {
    setSaving(true);
    try {
      await invoke('set_core_request_log', { enabled: true });
      await refresh();
    } catch (saveError) {
      setError(String(saveError));
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

      {error ? <p className="form-error">{error}</p> : null}

      {status ? (
        <>
          <label className="switch-row">
            <input
              type="checkbox"
              checked={status.settingsEnabled}
              disabled={saving}
              onChange={(event) => void persist({ settingsEnabled: event.target.checked })}
            />
            <span>{t('usage.archive.enable')}</span>
          </label>

          {readiness === 'awaiting-request-log' ? (
            <p className="form-error">
              {t('usage.archive.requestLogRequired')}{' '}
              <button type="button" className="link-button" disabled={saving} onClick={() => void enableRequestLog()}>
                {t('usage.archive.enableRequestLog')}
              </button>
            </p>
          ) : null}

          <div className="form-grid">
            <label>
              <span>{t('usage.archive.retentionDays')}</span>
              <input
                type="number"
                min={0}
                max={3650}
                value={status.retentionDays}
                disabled={saving}
                onChange={(event) => void persist({ retentionDays: Number(event.target.value) })}
              />
            </label>
            <label>
              <span>{t('usage.archive.maxTotalMb')}</span>
              <input
                type="number"
                min={0}
                value={status.maxTotalMb}
                disabled={saving}
                onChange={(event) => void persist({ maxTotalMb: Number(event.target.value) })}
              />
            </label>
            <label>
              <span>{t('usage.archive.maxBodyKb')}</span>
              <input
                type="number"
                min={16}
                value={status.maxBodyKb}
                disabled={saving}
                onChange={(event) => void persist({ maxBodyKb: Number(event.target.value) })}
              />
            </label>
          </div>

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
              <dt>{t('usage.archive.lastIngestedAt')}</dt>
              <dd>{status.lastIngestedAt || '—'}</dd>
            </div>
          </dl>

          <p className="usage-archive-path" title={status.databasePath}>
            {t('usage.archive.databasePath')}: <code>{status.databasePath}</code>
          </p>
          <p className="usage-archive-path" title={status.logsDirectory}>
            {t('usage.archive.logsDirectory')}: <code>{status.logsDirectory}</code>
          </p>
          {status.lastError ? <p className="form-error">{status.lastError}</p> : null}

          <div className="usage-archive-actions">
            <button type="button" disabled={saving} onClick={() => void refresh()}>
              <RefreshCw size={16} aria-hidden="true" />
              <span>{t('usage.archive.refresh')}</span>
            </button>
            <button type="button" disabled={saving} onClick={() => void invoke('open_core_logs_directory')}>
              <FolderOpen size={16} aria-hidden="true" />
              <span>{t('usage.archive.openLogs')}</span>
            </button>
            <button type="button" className="danger" disabled={saving} onClick={() => void clear()}>
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
  const [detail, setDetail] = useState<RequestArchiveDetail | null>(null);
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
          <button type="button" aria-label={t('usage.archive.close')} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        {loading ? <p>{t('usage.archive.loading')}</p> : null}
        {error ? <p className="form-error">{error}</p> : null}
        {!loading && !error && !detail ? <p>{t('usage.archive.notFound')}</p> : null}

        {detail ? (
          <>
            <p className="usage-archive-meta">
              {detail.method} {detail.endpoint} · {detail.model || '—'} · HTTP {detail.httpStatus} ·{' '}
              {detail.capturedAt}
              {detail.truncated ? ` · ${t('usage.archive.truncated')}` : ''}
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

            {section === 'system' ? (
              <pre className="usage-archive-content">{detail.systemPrompt || t('usage.archive.empty')}</pre>
            ) : null}

            {section === 'messages' ? (
              messages.length === 0 ? (
                <p>{t('usage.archive.empty')}</p>
              ) : (
                <ol className="usage-archive-messages">
                  {messages.map((message, index) => (
                    <li key={`${message.role}-${index}`}>
                      <span className="usage-archive-role">{message.role}</span>
                      {message.toolCalls.length > 0 ? (
                        <span className="usage-archive-tools">{message.toolCalls.join(', ')}</span>
                      ) : null}
                      <pre className="usage-archive-content">{message.text || t('usage.archive.empty')}</pre>
                    </li>
                  ))}
                </ol>
              )
            ) : null}

            {section === 'tools' ? (
              toolNames.length === 0 ? (
                <p>{t('usage.archive.empty')}</p>
              ) : (
                <>
                  <p className="usage-archive-meta">{toolNames.join(', ')}</p>
                  <pre className="usage-archive-content">{formatArchiveJson(detail.toolsJson)}</pre>
                </>
              )
            ) : null}

            {section === 'usage' ? (
              <pre className="usage-archive-content">
                {formatArchiveJson(detail.usageJson) || t('usage.archive.empty')}
              </pre>
            ) : null}

            {section === 'raw' ? (
              <>
                <h3>{t('usage.archive.rawRequest')}</h3>
                <pre className="usage-archive-content">
                  {formatArchiveJson(detail.apiRequest || detail.requestBody) || t('usage.archive.empty')}
                </pre>
                <h3>{t('usage.archive.rawResponse')}</h3>
                <pre className="usage-archive-content">
                  {detail.apiResponse || detail.responseBody || detail.apiErrorText || t('usage.archive.empty')}
                </pre>
              </>
            ) : null}
          </>
        ) : null}
      </section>
    </div>
  );
}
