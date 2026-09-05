# FORK.md — Fork 維護 SOT

這份文件是本 fork 的**單一事實來源（Source of Truth）**。任何關於「這個 fork 改了什麼、為什麼改、sync upstream 時要注意什麼」的問題，以本檔為準。

- 本檔隨 repo 版控，rebase 時會跟著走。
- 上游 `router-for-me/EasyCLIProxyAPI` 沒有這個檔案，因此**永遠不會衝突**。
- 修改任何 fork-specific 的東西時，**必須同步更新本檔**，否則本檔失去 SOT 資格。

> 設計過程與決策紀錄另存於 workspace 根目錄的 `plan.md`（未版控）。本檔只講「怎麼維護」，不重複設計細節。

---

## 1. Fork 身分

| 項目 | 值 |
| --- | --- |
| Upstream | `router-for-me/EasyCLIProxyAPI`（目前就是 `origin`） |
| 工作分支 | `feature/request-archive` |
| Fork 基底 | `8575f4e` — `fix(version): update cpa-gui version to 0.2.72` |
| Git 身分（repo local） | `fun-ed <git-ed@runbox.no>` |
| 核心版本綁定 | `core-version.txt` = `7.2.149` |
| 產出執行檔 | `bin-work/EasyCLIProxyAPI-fork` |
| 資料目錄 | `~/Library/Application Support/com.cpa.gui`（**與官方版共用**） |

### Fork commit 清單

依序疊在 upstream 之上，**每個 commit 都可獨立 revert**：

| Commit | 內容 | 可否單獨拿掉 |
| --- | --- | --- |
| `afa8120` | `feat(usage)`：完整請求歸檔（request archive） | 可 |
| `630fa48` | `chore(fork)`：改名為 `EasyCLIProxyAPI-fork` | 可 |

---

## 2. 架構分區

把 codebase 切成三區來想，維護成本差很多：

```
┌─ A 區：純新增（零衝突）────────────────────────────────┐
│  src-tauri/src/request_archive.rs                      │
│  src-tauri/src/request_archive/parser.rs               │
│  src-tauri/src/tests/request_archive.rs                │
│  src/services/requestArchive.ts                        │
│  src/pages/RequestArchivePanel.tsx                     │
│  src/i18n/locales/requestArchive.ts                    │
│  src/styles/requestArchive.css                         │
│  tests/requestArchive.test.ts                          │
│  FORK.md                                               │
└────────────────────────────────────────────────────────┘
┌─ B 區：整合點（rebase 時可能衝突，共 57 行）──────────┐
│  src-tauri/src/main.rs                    14 行        │
│  src/pages/UsageRecordsPage.tsx           24 行        │
│  src/i18n/locales/zh-CN.ts                 3 行        │
│  src/i18n/locales/en.ts                    2 行        │
│  src/i18n/ja.ts                            2 行        │
│  src-tauri/src/tests.rs                    1 行        │
│  README.md / README.zh-CN.md / README.ja.md 11 行      │
└────────────────────────────────────────────────────────┘
┌─ C 區：改名（11 行，全是單行字串替換）────────────────┐
│  src-tauri/tauri.conf.json  index.html                 │
│  src/App.tsx  src/pages/EasyModePage.tsx               │
│  scripts/portable.mjs  build.sh  run.sh                │
│  run.ps1  copy.ps1                                     │
└────────────────────────────────────────────────────────┘
```

**維護原則：新功能一律往 A 區放，B 區只留無法避免的接線。**

`CLIProxyAPI/`（Go 核心）**完全沒有 patch**，可以獨立照 upstream 升級。

---

## 3. Patch Inventory（B 區逐項）

rebase 出現衝突時，照這張表判斷該保留什麼。

### 3.1 `src-tauri/src/main.rs`（4 處，14 行）

| 位置 | 內容 | 衝突處理 |
| --- | --- | --- |
| `mod` 宣告區 | `mod request_archive;`（在 `provider_health` 之後） | 兩邊模組都保留，維持字母序 |
| `.manage(...)` 鏈 | `.manage(request_archive::RequestArchiveState::default())` | 加回去即可，順序不重要 |
| `setup` 內 spawn 區 | `spawn_blocking` 呼叫 `start_request_archive_ingester` | 放在 `start_usage_collector` 之後 |
| `generate_handler![]` | 7 個 `request_archive::*` 命令 | **最容易衝突**。兩邊的命令都要保留，一個都不能掉 |

### 3.2 `src/pages/UsageRecordsPage.tsx`（5 處，24 行）

| 位置 | 內容 | 衝突處理 |
| --- | --- | --- |
| import 區 | `import { RequestArchiveDetailDialog, RequestArchiveSettingsCard } from './RequestArchivePanel';` | 加回去 |
| `type UsageRecord` | 新增 `request_id: string;` | **必須保留**，沒有它整個詳情入口失效 |
| `data-management` 分頁 | `<UsageDataManagementView />` 外包一層 fragment，後面接 `<RequestArchiveSettingsCard />` | 若 upstream 重構分頁，把卡片掛回資料管理分頁即可 |
| `EventsView` state | `const [archiveRequestId, setArchiveRequestId] = useState('');` | 加回去 |
| 事件表 `<tr>` | `className` / `title` / `onClick` 三個 prop | **若 upstream 重構表格，這三個 prop 要重新掛回 `<tr>`** |
| `EventsView` 尾端 | 條件渲染 `<RequestArchiveDetailDialog>` | 放在 `<UsageEmpty />` 之後、欄位設定對話框之前 |

### 3.3 i18n（3 檔，7 行）

三個 locale 檔各加 1 行 import + 1 行 spread：

```ts
// src/i18n/locales/zh-CN.ts
import { requestArchiveZhCN } from './requestArchive';
export const zhCN = {
  ...requestArchiveZhCN,     // ← 這行
  ...

// src/i18n/locales/en.ts
import { requestArchiveEn } from './requestArchive';
export const en: Record<MessageKey, string> = {
  ...requestArchiveEn,       // ← 這行

// src/i18n/ja.ts
import { requestArchiveJa } from './locales/requestArchive';
export const jaOverrides = {
  ...requestArchiveJa,       // ← 這行
```

31 個 `usage.archive.*` key 的三語內容全在 `src/i18n/locales/requestArchive.ts`。**要加新文案就改那一個檔，不要往 locale 檔塞。**

`zh-TW` 由 `traditional.ts` 從 zh-CN 自動轉換，不需要維護。

### 3.4 C 區改名對照

| 檔案 | 原值 | fork 值 |
| --- | --- | --- |
| `src-tauri/tauri.conf.json` `productName` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `src-tauri/tauri.conf.json` `identifier` | `com.cpa.gui` | `com.cpa.gui.fork` |
| `src-tauri/tauri.conf.json` 視窗 `title` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `index.html` `<title>` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `src/App.tsx` 側邊欄 `<strong>` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `src/pages/EasyModePage.tsx` 品牌 `<strong>` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `scripts/portable.mjs` `outputBinary` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `build.sh` `BIN_OUT` / `run.sh` `APP_BIN` | `EasyCLIProxyAPI` | `EasyCLIProxyAPI-fork` |
| `run.ps1` `$AppBin` / `copy.ps1` `$BinOut` | `EasyCLIProxyAPI.exe` | `EasyCLIProxyAPI-fork.exe` |

**刻意沒改**（改了成本高於效益）：

- `src-tauri/src/tray.rs` 13 處狀態 tooltip、`i18n` 三語系的敘述文字 → 仍顯示 `EasyCLIProxyAPI`
- `scripts/portable.mjs` 的 `portable-app.json` `application: 'EasyCLIProxyAPI'` → 改了會讓 in-app 更新比對 upstream release 資產失效
- `Cargo.toml` `name = "cpa-gui"`、`package.json` `name` → 會連動 `target/release/cpa-gui`、`set-version.mjs`、`release.yml`

---

## 4. 不變式（Invariants）

改動 fork 時**不能破壞**這些前提，否則功能會靜默失效：

1. **資料目錄不可從 identifier 推導。**
   `core_runtime.rs` 的 `core_base_dir()` 把 `com.cpa.gui` 寫死。改 `tauri.conf.json` 的 identifier 不會動到資料目錄。若哪天 upstream 把它改成從 identifier 推導，fork 的 identifier 改名會**弄丟所有 auth / config / usage.db / requests.db**，屆時必須把 identifier 改回 `com.cpa.gui`。

2. **歸檔 DB 與 usage DB 完全分離。**
   `request-records/requests.db` 與 `usage-records/usage.db` 是兄弟目錄，不共用連線、不共用 schema，唯一的關聯是 `request_id` 字串。

3. **原始 log 檔永不刪除。** 保留策略只清 DB 列，不動 `logs/*.log`。

4. **歸檔設定存在歸檔 DB 自己的 `archive_metadata` 表**，不進 `GuiConfigFile`。這是刻意的：避免動到 `main.rs` 那條又長又容易衝突的 config 管線。

5. **新功能的 i18n / CSS 一律放 A 區獨立檔**，locale 檔與 `src/styles.css` 保持零改動或單行 spread。

6. **前端不得直接 `fetch`**，所有後端存取走 Tauri `invoke`；管理 API 走既有的 `management_request` 命令。

---

## 5. Fork 對 Upstream 的隱性依賴

以下依賴是**字串型或 schema 型的，編譯器抓不到**。upstream 改動它們時，fork 會**編譯通過但執行期壞掉**。每次 sync 後要抽查：

| 依賴 | 使用位置 | 壞掉的症狀 |
| --- | --- | --- |
| `invoke('set_core_request_log', { enabled })` | `RequestArchivePanel.tsx:65` | 「立即開啟 request-log」按鈕報錯 |
| `invoke('open_core_logs_directory')` | `RequestArchivePanel.tsx:184` | 「打開日誌目錄」按鈕無反應 |
| `usage_events.request_id` 欄位 | `UsageRecordsPage.tsx` 的 `UsageRecord` | 事件列不可點、詳情對話框開不起來 |
| 核心 request-log 檔案格式 | `request_archive/parser.rs` | 歸檔有列但欄位全空 |
| 核心 log 目錄解析規則 | `request_archive.rs` 的 `core_logs_directories()` | 歸檔完全沒有新記錄 |
| upstream CSS class（`panel`、`config-dialog-backdrop`、`form-grid`、`switch-row`、`link-button`、`danger`、`form-error`、`usage-data-management-panel`、`usage-data-management-heading`） | `RequestArchivePanel.tsx` | UI 版型跑掉但功能正常 |
| `MessageKey` 型別（`src/i18n/locales/zh-CN.ts` 匯出） | `requestArchive.ts`、`ja.ts`、`en.ts` | 這個**會**被 tsc 抓到，安全 |

**核心格式的權威來源**（唯讀參考，不要 patch）：

- `CLIProxyAPI/internal/logging/request_logger_format.go` — 區塊格式
- `CLIProxyAPI/internal/logging/request_logger_writer.go` — 檔名與區塊間距
- `CLIProxyAPI/internal/logging/`（`ResolveLogDirectory`）— log 目錄解析

---

## 6. Sync SOP

```bash
cd EasyCLIProxyAPI

# 0. 確認乾淨
git status --short

# 1. 取得 upstream
git fetch origin

# 2. rebase
git checkout feature/request-archive
git rebase origin/main
#    衝突只會出現在 §3 的清單裡，照表解
#    git add <file> && git rebase --continue

# 3. 核心版本是否跟著升？
cat core-version.txt        # upstream 若升版，這裡會變

# 4. 驗證（順序照下面跑，快的先跑）
bun install
bun run check                       # tsc --noEmit
bun test                            # 預期 156 pass / 1 既有失敗（見 §8）
bun run build                       # 確認 vite 能 bundle（含抽出的 CSS）
cd src-tauri && cargo test && cd ..  # 預期 332 passed / 0 failed

# 5. §5 隱性依賴抽查（見下方 checklist）

# 6. 編譯與實測
./build.sh
./run.sh
```

### Sync 後 checklist

- [ ] `grep -rn "set_core_request_log" src-tauri/src/core_config/commands.rs` 仍存在
- [ ] `grep -rn "open_core_logs_directory" src-tauri/src/management_api.rs` 仍存在
- [ ] `usage_events` 仍有 `request_id`（`src-tauri/src/usage.rs`）
- [ ] `git diff origin/main -- CLIProxyAPI/internal/logging/` 若核心也升版，確認 log 格式沒改
- [ ] 啟動 app：視窗標題是 `EasyCLIProxyAPI-fork`
- [ ] 用量統計 → 資料管理 → 「完整請求歸檔」卡片存在、狀態正常
- [ ] 發一筆真實請求 → 事件列可點 → 詳情對話框五個分頁都有內容

---

## 7. 環境重建（換機或重新 clone）

`.git/info/exclude` **不隨 repo 版控**，重新 clone 後要手動補：

```bash
cd EasyCLIProxyAPI
printf '\n# fork-local agent artifacts (not for upstream)\n/AGENTS.md\n/CLAUDE.md\n/openwiki/\n' >> .git/info/exclude
git config --local user.name  "fun-ed"
git config --local user.email "git-ed@runbox.no"
```

### 之後建立自己的 GitHub fork

```bash
git remote rename origin upstream
git remote add origin git@github.com:<你的帳號>/EasyCLIProxyAPI.git
git push -u origin feature/request-archive
# 之後 §6 的 `git fetch origin` 改成 `git fetch upstream`
#         `git rebase origin/main` 改成 `git rebase upstream/main`
```

> 注意：`.github/workflows/release.yml` 是 upstream 的發佈流程，會產出 upstream 命名的資產、並參照 `target/release/cpa-gui`。fork 目前**只做本機編譯**，沒有啟用 CI。若要啟用，需另行處理資產命名與 signing。

---

## 8. 已知問題

| 問題 | 狀態 |
| --- | --- |
| `tests/i18n.test.ts` 失敗：`jaOverrides` 比 `zhCN` 少 50 個 key | **upstream 既有問題**，非本 fork 造成。fork 前後缺口都是 50，且沒有任何 `usage.archive.*` 在缺漏清單裡。**不要為了讓它綠掉而亂補 key**，先確認缺口數字仍是 50 即可 |
| 歸檔功能未做端到端實測 | 解析器正確性目前只由依 Go writer 原始碼建構的合成樣本證明。建議至少實測一筆 Claude 形狀、一筆 OpenAI 形狀的請求 |
| `usage_events.request_id` 是否在所有 provider 路徑都與 log 檔名 id 一致 | 未逐一驗證 |
| Antigravity / Codex WebSocket timeline 的 usage 擷取 | 靠泛用遞迴搜尋 `usage` / `usageMetadata`，可能不完整 |
| identifier 改名的孤兒登入項目 | 若先前開過「開機自動啟動」，舊的 `com.cpa.gui` 登入項目要手動移除後重開一次 |
| `bin-work/` 舊檔 | `portable.mjs` 只清 legacy 的 `cpa-gui`，舊的 `EasyCLIProxyAPI` 要自行刪除 |
| 歸檔內容含 system prompt 與完整 payload | DB 未加密，等同敏感資料落地，自行控管檔案權限 |

---

## 9. 功能速查：完整請求歸檔

| 項目 | 值 |
| --- | --- |
| DB 路徑 | `<core base dir>/request-records/requests.db` |
| macOS 實際位置 | `~/Library/Application Support/com.cpa.gui/request-records/requests.db` |
| 開發模式位置 | `src-tauri/target/debug/request-records/requests.db` |
| 主表 | `request_records`（38 欄）、`archive_metadata`（設定） |
| 掃描間隔 | 5 秒，單次最多 200 檔 |
| 去重鍵 | `source_file` UNIQUE + `source_fingerprint = "{len}:{mtime_ms}"` |
| 預設保留 | 30 天 / 5120 MB / 單欄位 1024 KB（皆可在 UI 調整，`0` = 不限制） |
| 前端事件 | `request-archive-updated` |
| 啟用路徑 | 用量統計 → 資料管理 → 完整請求歸檔 → 勾選 + 「立即開啟 request-log」 |
| 查看路徑 | 用量統計 → 事件列表 → 點任一列 |

Tauri 命令：`get_request_archive_status` / `save_request_archive_settings` / `query_request_archive_records` / `get_request_archive_record` / `get_request_archive_record_by_request_id` / `get_request_archive_models` / `clear_request_archive`

直接查 DB：

```bash
sqlite3 "$HOME/Library/Application Support/com.cpa.gui/request-records/requests.db" \
  "SELECT captured_at, model, http_status, total_tokens, request_id
     FROM request_records ORDER BY captured_at_ms DESC LIMIT 20;"
```

---

## 10. 變更本檔的時機

以下任一情況發生，**必須更新本檔**：

- 新增／移除 fork commit
- 動到 B 區或 C 區任何一行
- 新增對 upstream 的隱性依賴（新的 `invoke` 字串、新的 upstream schema 欄位、新的 upstream CSS class）
- rebase 後 base commit 變更（更新 §1 的 Fork 基底）
- 已知問題被修掉或新增
