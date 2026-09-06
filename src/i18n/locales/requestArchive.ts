// Fork-local feature: full request archive.
// Kept in its own module so syncing upstream only conflicts on a single spread
// line inside each locale file instead of a 30+ line block.

export const requestArchiveZhCN = {
  'usage.archive.title': '完整请求归档',
  'usage.archive.description':
    '解析核心的 request-log 日志，把系统提示词、消息、工具、用量与 HTTP 状态存入独立的 SQLite 数据库。',
  'usage.archive.enable': '启用完整请求归档',
  'usage.archive.requestLogRequired': '核心的 request-log 未开启，归档不会产生任何记录。',
  'usage.archive.enableRequestLog': '立即开启 request-log',
  'usage.archive.requestLogSwitch': '开启核心 request-log（归档的数据来源）',
  'usage.archive.commercialModeBlocked':
    '内核的商用模式（commercial-mode）已开启，它会完全跳过请求日志中间件，request-log 不会生效。请到「高级设置」关闭商用模式并重启内核。',
  'usage.archive.retentionDays': '保留天数（0 表示不限制）',
  'usage.archive.maxTotalMb': '数据库上限 MB（0 表示不限制）',
  'usage.archive.maxBodyKb': '单字段上限 KB',
  'usage.archive.recordCount': '已归档记录',
  'usage.archive.databaseSize': '数据库占用',
  'usage.archive.logsSize': '日志目录占用',
  'usage.archive.lastIngestedAt': '最近写入时间',
  'usage.archive.databasePath': '数据库路径',
  'usage.archive.logsDirectory': '核心日志目录',
  'usage.archive.refresh': '刷新状态',
  'usage.archive.openLogs': '打开日志目录',
  'usage.archive.clear': '清空归档',
  'usage.archive.compact': '压缩数据库',
  'usage.archive.purgeLogs': '清理已归档日志',
  'usage.archive.purgeLogsConfirm':
    '删除核心日志目录下已经归档的请求日志，以及已结束请求残留的临时目录。尚未归档的文件会保留。要继续吗？',
  'usage.archive.maintenanceResult': '处理完成：清理 {files} 项，释放 {freed}，跳过未归档 {skipped} 项。',
  'usage.archive.clearConfirm': '确定清空全部请求归档记录吗？原始日志文件会保留。',
  'usage.archive.detailTitle': '请求完整内容',
  'usage.archive.close': '关闭',
  'usage.archive.loading': '正在读取归档内容…',
  'usage.archive.notFound':
    '未找到该请求的归档内容。归档只覆盖启用之后产生的请求；若曾清空归档或删除来源日志，更早的请求就无法显示。事件列表本身来自另一个数据库，因此仍会列出这些请求。',
  'usage.archive.empty': '（无内容）',
  'usage.archive.truncated': '内容已按上限截断',
  'usage.archive.rawRequest': '原始请求',
  'usage.archive.rawResponse': '原始响应',
  'usage.archive.viewDetail': '查看完整请求内容',
  'usage.archive.section.messages': '消息',
  'usage.archive.section.system': '系统提示词',
  'usage.archive.section.tools': '工具',
  'usage.archive.section.usage': '用量明细',
  'usage.archive.section.raw': '原始报文',
} as const;

export type RequestArchiveMessageKey = keyof typeof requestArchiveZhCN;

export const requestArchiveEn: Record<RequestArchiveMessageKey, string> = {
  'usage.archive.title': 'Full Request Archive',
  'usage.archive.description':
    "Parses the core's request-log transcripts and stores system prompts, messages, tools, usage, and HTTP status in a separate SQLite database.",
  'usage.archive.enable': 'Enable full request archive',
  'usage.archive.requestLogRequired':
    'The core has request-log disabled, so the archive will not collect anything.',
  'usage.archive.enableRequestLog': 'Enable request-log now',
  'usage.archive.requestLogSwitch': 'Enable the core request-log (the archive data source)',
  'usage.archive.commercialModeBlocked':
    'The core runs in commercial mode, which skips the request logging middleware entirely, so request-log has no effect. Turn commercial mode off in Advanced Settings and restart the core.',
  'usage.archive.retentionDays': 'Retention days (0 means unlimited)',
  'usage.archive.maxTotalMb': 'Database cap in MB (0 means unlimited)',
  'usage.archive.maxBodyKb': 'Per-field cap in KB',
  'usage.archive.recordCount': 'Archived records',
  'usage.archive.databaseSize': 'Database size',
  'usage.archive.logsSize': 'Logs directory size',
  'usage.archive.lastIngestedAt': 'Last ingest',
  'usage.archive.databasePath': 'Database path',
  'usage.archive.logsDirectory': 'Core logs directory',
  'usage.archive.refresh': 'Refresh status',
  'usage.archive.openLogs': 'Open logs directory',
  'usage.archive.clear': 'Clear archive',
  'usage.archive.compact': 'Compact database',
  'usage.archive.purgeLogs': 'Purge archived logs',
  'usage.archive.purgeLogsConfirm':
    'Delete request logs that have already been archived, plus scratch directories left by finished requests. Files not yet archived are kept. Continue?',
  'usage.archive.maintenanceResult': 'Done: {files} removed, {freed} freed, {skipped} not yet archived and kept.',
  'usage.archive.clearConfirm':
    'Clear every archived request record? The original log files are kept.',
  'usage.archive.detailTitle': 'Full Request Content',
  'usage.archive.close': 'Close',
  'usage.archive.loading': 'Loading archived content…',
  'usage.archive.notFound':
    'No archived content for this request. The archive only covers requests made after it was enabled, and clearing it or deleting the source logs removes earlier ones. The event list comes from a separate database, so those requests still appear there.',
  'usage.archive.empty': '(empty)',
  'usage.archive.truncated': 'Content truncated by the per-field cap',
  'usage.archive.rawRequest': 'Raw request',
  'usage.archive.rawResponse': 'Raw response',
  'usage.archive.viewDetail': 'View full request content',
  'usage.archive.section.messages': 'Messages',
  'usage.archive.section.system': 'System prompt',
  'usage.archive.section.tools': 'Tools',
  'usage.archive.section.usage': 'Usage detail',
  'usage.archive.section.raw': 'Raw payloads',
};

export const requestArchiveJa: Record<RequestArchiveMessageKey, string> = {
  'usage.archive.title': '完全リクエストアーカイブ',
  'usage.archive.description':
    'コアの request-log を解析し、システムプロンプト・メッセージ・ツール・使用量・HTTP ステータスを独立した SQLite データベースに保存します。',
  'usage.archive.enable': '完全リクエストアーカイブを有効化',
  'usage.archive.requestLogRequired':
    'コアの request-log が無効なため、アーカイブには何も記録されません。',
  'usage.archive.enableRequestLog': 'request-log を今すぐ有効化',
  'usage.archive.requestLogSwitch': 'コアの request-log を有効化（アーカイブのデータ元）',
  'usage.archive.commercialModeBlocked':
    'コアが商用モードで動作しているため、リクエストログのミドルウェアが登録されず request-log は機能しません。詳細設定で商用モードを無効にし、コアを再起動してください。',
  'usage.archive.retentionDays': '保持日数（0 は無制限）',
  'usage.archive.maxTotalMb': 'データベース上限 MB（0 は無制限）',
  'usage.archive.maxBodyKb': 'フィールドごとの上限 KB',
  'usage.archive.recordCount': 'アーカイブ済み件数',
  'usage.archive.databaseSize': 'データベース使用量',
  'usage.archive.logsSize': 'ログディレクトリ使用量',
  'usage.archive.lastIngestedAt': '最終取り込み',
  'usage.archive.databasePath': 'データベースのパス',
  'usage.archive.logsDirectory': 'コアのログディレクトリ',
  'usage.archive.refresh': '状態を更新',
  'usage.archive.openLogs': 'ログディレクトリを開く',
  'usage.archive.clear': 'アーカイブを消去',
  'usage.archive.compact': 'データベースを圧縮',
  'usage.archive.purgeLogs': 'アーカイブ済みログを削除',
  'usage.archive.purgeLogsConfirm':
    'すでにアーカイブされたリクエストログと、完了したリクエストの一時ディレクトリを削除します。未アーカイブのファイルは保持されます。続行しますか？',
  'usage.archive.maintenanceResult': '完了：{files} 件を削除、{freed} を解放、未アーカイブ {skipped} 件は保持。',
  'usage.archive.clearConfirm':
    'アーカイブ済みのリクエスト記録をすべて削除しますか？元のログファイルは保持されます。',
  'usage.archive.detailTitle': 'リクエストの完全な内容',
  'usage.archive.close': '閉じる',
  'usage.archive.loading': 'アーカイブ内容を読み込んでいます…',
  'usage.archive.notFound':
    'このリクエストのアーカイブが見つかりません。アーカイブは有効化した後のリクエストのみを対象とし、消去やログ削除を行うとそれ以前の分は残りません。イベント一覧は別のデータベースを参照するため、引き続き表示されます。',
  'usage.archive.empty': '（内容なし）',
  'usage.archive.truncated': '上限により内容が切り詰められました',
  'usage.archive.rawRequest': '元のリクエスト',
  'usage.archive.rawResponse': '元のレスポンス',
  'usage.archive.viewDetail': 'リクエストの完全な内容を表示',
  'usage.archive.section.messages': 'メッセージ',
  'usage.archive.section.system': 'システムプロンプト',
  'usage.archive.section.tools': 'ツール',
  'usage.archive.section.usage': '使用量の内訳',
  'usage.archive.section.raw': '生のペイロード',
};
