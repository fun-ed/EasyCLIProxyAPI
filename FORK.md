# FORK.md — Fork 維護 SOT

這份文件是本 fork 的**單一事實來源（Source of Truth）**。任何關於「這個 fork 改了什麼、為什麼改、sync upstream 時要注意什麼」的問題，以本檔為準。

- 本檔隨 repo 版控，rebase 時會跟著走。
- 上游 `router-for-me/EasyCLIProxyAPI` 沒有這個檔案，因此**永遠不會衝突**。
- 修改任何 fork-specific 的東西時，**必須同步更新本檔**，否則本檔失去 SOT 資格。

> 設計過程與決策紀錄另存於 workspace 根目錄的 `plan.md`（未版控）。本檔只講「怎麼維護」，不重複設計細節。

---

## 0. 固定流程（日常只需要看這一節）

```bash
cd <workspace 根目錄>          # 例如 ~/temp/cliproxy-api

./sync-and-build.sh --check    # 1. 先看 upstream 有沒有更新（只 fetch，不改任何東西）
./sync-and-build.sh --app      # 2. 同步 + 建置 + 驗證 + 打包 + 產生 .app
open EasyCLIProxyAPI/src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

第 1 步可省略，直接跑第 2 步也會自己 fetch。`--check` 的離開碼可供排程判斷：`0` = 已是最新，`10` = 有更新待套用。

> **一定要加 `--app`。** 不加只會產出 `bin-work/` 的可攜版，那個版本**自帶一份空白 profile**（沒有你的憑證、沒有用量記錄、歸檔永遠 0 筆）。原因見 §9.1。

只改了程式碼、想最快看到結果：

```bash
./sync-and-build.sh --app-only    # 跳過 git 與核心/面板，只驗證 + 出 .app，約 1 分鐘
```

腳本會自己 fetch、rebase、建置、驗證、打包。細節見 §6，衝突處理見 §6.2，手動指令見 §6.4。

### 成功長這樣

結尾必須看到 Summary 區塊，且沒有 `FAILED:`：

```
==> Summary
    CLIProxyAPI: unchanged (5208aec7)
    Cli-Proxy-API-Management-Center: unchanged (87b7ce1)
    EasyCLIProxyAPI: no rebase needed (73c8b36)
    CLIProxyAPI: build OK
    panel: installed to .../cpa-core/static/management.html
    EasyCLIProxyAPI: bun test had the known i18n failure only
    EasyCLIProxyAPI: verify OK
    EasyCLIProxyAPI: packaged .../bin-work/EasyCLIProxyAPI-fork
Done.
```

`bun test had the known i18n failure only` 是**正常的**，那是 upstream 既有問題（§8）。

### 預期耗時

| 情境 | 實測 |
| --- | --- |
| 無 upstream 更新，增量重建 | 約 1 分鐘 |
| upstream 有更新，需 rebase + 完整重編 | 約 5 分鐘 |

### 出事時的三張牌

| 狀況 | 動作 |
| --- | --- |
| rebase 衝突 | 照 §3 的 patch inventory 解，然後 `./sync-and-build.sh --no-sync`（§6.2） |
| 歸檔功能壞掉但想先用 app | `CPA_FORK_ARCHIVE=0 ./EasyCLIProxyAPI/run.sh`（§1.1） |
| 腳本本身壞掉 | 走 §6.3 的手動流程 |

### 這個流程的驗證狀態

已在兩種情境實跑通過：

1. **真實 upstream 前進**：upstream 從 `8575f4e` 推進 3 個 commit 到 `584c63b`（cpa-gui 0.2.72 → 0.2.73、核心 7.2.149 → 7.2.151），6 個 fork commit **零衝突** rebase，完成打包。
2. **無更新增量重建**：三個 repo 皆為最新，1 分 08 秒完成全流程。

---

## 1. Fork 身分

| 項目 | 值 |
| --- | --- |
| Upstream | `router-for-me/EasyCLIProxyAPI`（目前就是 `origin`） |
| 工作分支 | `feature/request-archive` |
| Fork 基底 | `584c63b` — `fix(version): update cpa-gui version to 0.2.73` |
| Git 身分（repo local） | `fun-ed <git-ed@runbox.no>` |
| 核心版本綁定 | `core-version.txt` = `7.2.151` |
| 產出執行檔 | `bin-work/EasyCLIProxyAPI-fork` |
| 資料目錄 | `~/Library/Application Support/com.cpa.gui`（**與官方版共用**） |

### Fork commit 清單

依序疊在 upstream 之上，**每個 commit 都可獨立 revert**：

| Commit | 內容 | 可否單獨拿掉 |
| --- | --- | --- |
| `07f2ae4` | `feat(usage)`：完整請求歸檔（request archive） | 可 |
| `5c5004f` | `chore(fork)`：改名為 `EasyCLIProxyAPI-fork` | 可 |
| `ddefcbf` | `feat(fork)`：`CPA_FORK_ARCHIVE` 總開關 | 可 |

## 1.1 緊急停用（kill switch）

sync 之後功能壞掉時，**不需要 revert commit**，設環境變數就能回到 upstream 行為：

```bash
CPA_FORK_ARCHIVE=0 ./bin-work/EasyCLIProxyAPI-fork
```

接受的關閉值（不分大小寫、自動去空白）：`0`、`false`、`off`、`no`。其他值或未設定 = 功能啟用。

關閉後的效果：

| 面向 | 行為 |
| --- | --- |
| 採集器 | 不啟動，stderr 印出一行說明 |
| `get_request_archive_status` | 直接回 `forkDisabled: true`，不碰 DB |
| 設定卡片 | 不渲染 |
| 事件列 | 不可點、無 hover 游標 |
| 詳情對話框 | 不渲染 |
| 既有 `requests.db` | 保留不動，重新開啟後資料還在 |

實作只有兩個檢查點（`request_archive.rs` 的 ingester 啟動與 status 命令），UI 判斷全在 fork 自有的 `RequestArchivePanel.tsx`，因此在 upstream 檔案只多 1 行。

---

## 2. 架構分區

把 codebase 切成三區來想，維護成本差很多：

```
┌─ A 區：純新增（零衝突）────────────────────────────────┐
│  src-tauri/src/model_prices_dev.rs                     │
│  src-tauri/src/tests/model_prices_dev.rs               │
│  src-tauri/src/request_archive.rs                      │
│  src-tauri/src/request_archive/parser.rs               │
│  src-tauri/src/tests/request_archive.rs                │
│  src/services/requestArchive.ts                        │
│  src/pages/RequestArchivePanel.tsx                     │
│  src/i18n/locales/requestArchive.ts                    │
│  src/styles/requestArchive.css                         │
│  tests/requestArchive.test.ts                          │
│  scripts/fork-sync-and-build.sh                        │
│  FORK.md  AGENTS.md  README.fork.md                    │
└────────────────────────────────────────────────────────┘
┌─ B 區：整合點（rebase 時可能衝突，共 58 行）──────────┐
│  src-tauri/src/main.rs                    14 行        │
│  src/pages/UsageRecordsPage.tsx           25 行        │
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

### 3.2 `src/pages/UsageRecordsPage.tsx`（6 處，25 行）

| 位置 | 內容 | 衝突處理 |
| --- | --- | --- |
| import 區 | `import { RequestArchiveDetailDialog, RequestArchiveSettingsCard, useRequestArchiveAvailable } from './RequestArchivePanel';` | 加回去 |
| `type UsageRecord` | 新增 `request_id: string;` | **必須保留**，沒有它整個詳情入口失效 |
| `data-management` 分頁 | `<UsageDataManagementView />` 外包一層 fragment，後面接 `<RequestArchiveSettingsCard />` | 若 upstream 重構分頁，把卡片掛回資料管理分頁即可 |
| `EventsView` state | `const [archiveRequestId, setArchiveRequestId] = useState('');` 與其下一行的 `const archiveAvailable = useRequestArchiveAvailable() !== false;` | 兩行都要加回去 |
| 事件表 `<tr>` | `className` / `title` / `onClick` 三個 prop，都以 `archiveAvailable &&` 起頭 | **若 upstream 重構表格，這三個 prop 要重新掛回 `<tr>`** |
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

### 6.1 一行指令（平常用這個，摘要見 §0）

```bash
cd <workspace 根目錄>
./sync-and-build.sh
```

實體檔案是 `EasyCLIProxyAPI/scripts/fork-sync-and-build.sh`（隨 repo 版控，換機不會遺失），workspace 根目錄的 `sync-and-build.sh` 只是指過去的 symlink。兩個入口行為相同。

它會依序做：

| 步驟 | 內容 | 安全機制 |
| --- | --- | --- |
| 1 | `CLIProxyAPI` fetch + **fast-forward only** | 偵測到任何本機 commit 就中止（守護 §4 不變式「核心零 patch」） |
| 2 | `Cli-Proxy-API-Management-Center` fetch + fast-forward only | 同上 |
| 3 | `EasyCLIProxyAPI` fetch + `rebase origin/main` | 衝突時停下並印出 §3 的對照指引，**絕不自動解衝突** |
| 4 | 核心 `go build` 編譯檢查 + gofmt 檢查 | 只警告不阻擋 gofmt |
| 5 | 面板 `bun run build` → 安裝到 `<core install dir>/static/management.html` | 內容相同則跳過複製 |
| 6 | 把核心 config 的 `disable-auto-update-panel` 釘成 `true` | 否則核心每 3 小時會用 GitHub 版覆蓋你的本機 build |
| 7 | fork `bun install` / `bun run check` / `bun test` / `cargo test` | i18n 既有失敗只警告；cargo 失敗會重試一次以排除 §8 的 flake |
| 8 | fork `./build.sh` → `bin-work/EasyCLIProxyAPI-fork`（可攜版） | — |
| 9 | 加 `--app` 時額外 `bun tauri build` → `.app` bundle | 這才是共用真實 profile 的版本 |

所有 repo 在動之前都會檢查工作區乾淨，有未提交改動就中止。

常用選項：

```bash
./sync-and-build.sh --check        # 只回報 upstream 有無更新（exit 0=最新, 10=有更新）
./sync-and-build.sh --app          # 同步 + 全部建置 + 額外產生 .app bundle
./sync-and-build.sh --app-only     # 只驗證 + 出 .app（跳過 git、核心、面板），約 1 分鐘
./sync-and-build.sh --no-sync      # 不動 git，只重建
./sync-and-build.sh --no-package   # 跑完測試但跳過慢的 release build
./sync-and-build.sh --skip-core --skip-panel   # 只處理 fork
./sync-and-build.sh --help
```

環境變數：`FORK_BRANCH`（預設 `feature/request-archive`）、`CPA_CORE_INSTALL_DIR`。

### 6.2 rebase 衝突時

腳本會停在衝突並印出指引。照 §3 的 patch inventory 解完後：

```bash
git -C EasyCLIProxyAPI add <file>
git -C EasyCLIProxyAPI rebase --continue
./sync-and-build.sh --no-sync      # 接續跑完建置與驗證
```

放棄：`git -C EasyCLIProxyAPI rebase --abort`

### 6.3 手動流程（腳本壞掉時的後備）

```bash
cd EasyCLIProxyAPI
git status --short
git fetch origin
git checkout feature/request-archive
git rebase origin/main
cat core-version.txt                 # upstream 若升版，這裡會變
bun install
bun run check
bun test
bun run build
cd src-tauri && cargo test && cd ..
```

### 6.4 手動建置 app（兩種產物，用途不同）

| 產物 | 指令 | 資料目錄 | 用途 |
| --- | --- | --- | --- |
| **`.app` bundle** | `bun tauri build` | **共用真實 profile** | **平常用這個**，能看到你的憑證與用量 |
| 可攜版 binary | `./build.sh` | 自帶空白 profile | 分發、或要乾淨環境測試 |

**`.app`（推薦）**

```bash
cd EasyCLIProxyAPI
bun install
bun tauri build
# 產物：
#   src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
#   src-tauri/target/release/bundle/dmg/EasyCLIProxyAPI-fork_<版本>_aarch64.dmg

open src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

**可攜版**

```bash
cd EasyCLIProxyAPI
./build.sh          # 內含 bun install + bun tauri build --no-bundle + portable 打包
./run.sh            # 等同執行 bin-work/EasyCLIProxyAPI-fork
```

**啟動前務必先關掉另一個版本**，兩者會搶 port 8317：

```bash
pkill -x cpa-gui; sleep 3
pgrep -fl "cpa-gui|cli-proxy-api" || echo "已全部關閉"
```

帶著 kill switch 啟動（歸檔壞掉時）：

```bash
CPA_FORK_ARCHIVE=0 open src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

耗時參考：`.app` 建置約 1 分鐘（增量），首次或 `cargo clean` 後約 4 分鐘。

### Sync 後 checklist

- [ ] `grep -rn "set_core_request_log" src-tauri/src/core_config/commands.rs` 仍存在
- [ ] `grep -rn "open_core_logs_directory" src-tauri/src/management_api.rs` 仍存在
- [ ] `usage_events` 仍有 `request_id`（`src-tauri/src/usage.rs`）
- [ ] `git diff origin/main -- CLIProxyAPI/internal/logging/` 若核心也升版，確認 log 格式沒改
- [ ] 啟動 app：視窗標題是 `EasyCLIProxyAPI-fork`
- [ ] 用量統計 → 資料管理 → 「完整請求歸檔」卡片存在、狀態正常
- [ ] 發一筆真實請求 → 事件列可點 → 詳情對話框五個分頁都有內容
- [ ] 若上一項失敗：先用 `CPA_FORK_ARCHIVE=0` 啟動確認 app 其餘功能正常，再回頭修（§1.1）

---

## 7. 環境重建（換機或重新 clone）

`.git/info/exclude` **不隨 repo 版控**，重新 clone 後要手動補：

```bash
cd EasyCLIProxyAPI
printf '\n# fork-local agent artifacts (not for upstream)\n/AGENTS.md\n/CLAUDE.md\n/openwiki/\n' >> .git/info/exclude
git config --local user.name  "fun-ed"
git config --local user.email "git-ed@runbox.no"
```

`CLIProxyAPI/`（Go 核心）也要做同樣的本機忽略。核心**必須永遠保持乾淨的 upstream 狀態**（§4 不變式），所以 OpenWiki 產物不進版控：

```bash
cd CLIProxyAPI
printf '\n# local agent artifacts (not for upstream)\n/openwiki/\n/AGENTS.md.bak.*\n/CLAUDE.md.bak.*\n' >> .git/info/exclude
```

若 OpenWiki 工具又往 `CLIProxyAPI/AGENTS.md` 或 `CLAUDE.md` 寫入區塊，**還原它們**（先備份再 `git checkout --`）。原因有二：

1. 核心一旦有本機改動，之後 `git pull` 就會衝突。
2. 該 repo 的 CI 有 `agents-md-guard.yml`，**任何碰到 `AGENTS.md` 的 PR 會被自動關閉**。

相同內容在 workspace 根目錄的 `AGENTS.md` / `CLAUDE.md` 已有一份，`openwiki/` 也可由工具重新產生，還原不會損失資訊。

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
| `tests/i18n.test.ts` 失敗：`jaOverrides` 比 `zhCN` 少 50 個 key | **upstream 既有問題**，非本 fork 造成。fork 前後缺口都是 50，且沒有任何 `usage.archive.*` 在缺漏清單裡。**不要為了讓它綠掉而亂補 key**，先確認缺口數字仍是 50 即可。sync 腳本會辨識這一筆並只發警告 |
| `tests::instance_lock::app_instance_guard_rejects_a_second_copy_and_releases_on_drop` 偶發失敗 | **upstream 既有 flake**。該測試用固定目錄 `agent_test_home("instance-lock")`，全套並行時偶爾輸給 OS 檔案鎖的釋放時序。單獨執行必過；連跑三次全套也全過。sync 腳本失敗時會自動重試一次 |
| 成功請求（含 usage / token 數）未實測 | 解析器已用**真實核心產出的 log** 驗證三種 provider 形狀（見 §8.1），但那三筆都是 HTTP 400 失敗請求。`usage_json` 與 token 欄位的擷取仍只有合成樣本覆蓋，需要一筆真實成功請求才算完整 |
| UI 渲染未實測 | 詳情對話框五個分頁的實際渲染尚未在真實資料上目視確認 |
| `usage_events.request_id` 是否在所有 provider 路徑都與 log 檔名 id 一致 | 未逐一驗證 |
| Antigravity / Codex WebSocket timeline 的 usage 擷取 | 靠泛用遞迴搜尋 `usage` / `usageMetadata`，可能不完整 |
| identifier 改名的孤兒登入項目 | 若先前開過「開機自動啟動」，舊的 `com.cpa.gui` 登入項目要手動移除後重開一次 |
| `bin-work/` 舊檔 | `portable.mjs` 只清 legacy 的 `cpa-gui`，舊的 `EasyCLIProxyAPI` 要自行刪除 |
| 歸檔內容含 system prompt 與完整 payload | DB 未加密，等同敏感資料落地，自行控管檔案權限 |

---

### 8.1 真實 log 迴歸測試

`src-tauri/src/tests/fixtures/real-core-*.txt` 是從 **CLIProxyAPI 7.2.151 實際執行產出**的 transcript，不是手寫樣本。

> 副檔名刻意用 `.txt` 而非 `.log`：upstream 的 `.gitignore:12` 有 `*.log`，用 `.log` 會被靜默排除在版控外，新 clone 會因 `include_str!` 找不到檔案而編譯失敗。**新增 fixture 時務必用 `.txt`。**用隔離實例（獨立 port、獨立 auth-dir、`request-log: true`）產生，未動到正式環境、未消耗任何額度。

由兩個測試守護：

- `parses_a_transcript_captured_from_a_live_core` — 逐欄比對 Claude 形狀
- `ingests_live_core_transcripts_across_provider_shapes` — 完整 ingest → DB，涵蓋 Claude / OpenAI / Gemini 三種形狀

**核心的 writer 格式一改，這兩個測試就會紅**，不會靜默產生空白歸檔列。

重新產生 fixture 的方式：起一個隔離核心（見 §6.3 精神：獨立 port 與 auth-dir、`request-log: true`），送出各形狀請求，把 `<auth-dir>/logs/*.log` 複製進 fixtures 並更新測試中的檔名與斷言。

### 8.2 已由真實 log 抓到並修掉的問題

| 問題 | 說明 |
| --- | --- |
| Gemini 形狀的 `model` 欄位是空的 | Gemini 原生 API 把模型放在 URL 路徑（`/v1beta/models/gemini-3-pro:generateContent`），不在 request body 的 `model` 鍵。已新增 `parser::model_from_url()` 作為 fallback，並用 `model_is_recovered_from_gemini_style_paths_only` 覆蓋邊界（其他 surface 不得亂猜） |

### 8.3 憑證外洩面（已查證）

核心的 `util.MaskSensitiveHeaderValue` 只遮蔽 header 名稱含 `authorization` / `api-key` / `apikey` / `token` / `secret` 的欄位，**其餘 header 一律原文寫入 log，因此也會原文進歸檔**（例如 `Cookie`）。

`Authorization` 是**部分遮罩**而非全遮罩：實測寫出的是 `Bearer e2e-...-key` 這種頭尾保留形式，完整憑證不會落地，但前後綴會。

加上 body 全文本來就會存，因此歸檔 DB 應視為敏感資料，自行控管檔案權限。

---

## 9. 功能速查：models.dev 價格來源

upstream 的價格表是手工維護的（`src-tauri/resources/model_prices.json`，57 個模型），更新落後於新模型發布。實測 `claude-opus-5` 不在表內，導致 655 筆請求完全無法計價，而遠端鏡像與內建表是同一份、同一個日期，按「同步价格」也沒用。

本 fork 把 **models.dev**（3326 個模型）加為主要來源：

```
models.dev/api.json  →  轉成既有 catalog 格式  →  upstream 既有的解析與儲存
        ↓ 失敗
upstream GitHub 價格表
        ↓ 失敗
內建 model_prices.json
```

手動設定的價格永遠優先，不受同步影響（upstream 既有行為）。

**provider 優先序是關鍵**：models.dev 是 provider → models 的巢狀結構，同一個 model id 會出現在多個 provider 下且**價格不同**（`gpt-5.6-terra` 在 `openai` 是 2/12，在轉售商是 2.5/15）。直接迭代 `HashMap` 會因為 Rust 隨機化 hash 順序而**每次同步得到不同價格**。

因此 `model_prices_dev.rs` 以固定順序走訪：`FIRST_PARTY_PROVIDERS` 清單優先，其餘依字母序，同一 model id 第一個 命中者勝出。`first_party_vendors_outrank_resellers_and_the_result_is_stable` 會連跑 8 次比對輸出是否逐字節相同。

models.dev 的分層計價（`tiers` / `context_over_200k`）不會匯入，因為用量資料庫沒有對應欄位，只取基礎級距。

---

## 10. 功能速查：完整請求歸檔

| 項目 | 值 |
| --- | --- |
| DB 路徑 | `<core base dir>/request-records/requests.db` |

### 10.1 資料目錄依「怎麼打包」而不同（重要）

`core_base_dir()`（`core_runtime.rs:1361`）**只有執行檔位於 `.app` bundle 內時**才回傳共用目錄，否則回傳執行檔所在目錄：

| 建置方式 | 執行檔位置 | 資料目錄（含 `request-records/`、`usage-records/`、`oauth/`） |
| --- | --- | --- |
| `./build.sh`（`--no-bundle` 可攜版） | `bin-work/EasyCLIProxyAPI-fork` | **`bin-work/`** — 自成一個**全新空白 profile** |
| `bun tauri build`（`.app` bundle） | `.../bundle/macos/EasyCLIProxyAPI-fork.app` | `~/Library/Application Support/com.cpa.gui` — **與正式版共用** |
| `bun tauri dev` | `src-tauri/target/debug/` | `src-tauri/target/debug/` |

**這是 upstream 的設計（可攜版本來就該自我包含），不是 fork 的 bug。**

實務影響：用 `./build.sh` 的可攜版測試時，會看到**沒有憑證、沒有用量記錄、歸檔 0 筆**，因為那是全新 profile。要用你真實的資料測，必須建 `.app`（指令見 §6.4，或直接 `./sync-and-build.sh --app-only`）。

`.app` 版與正式版共用 `~/Library/Application Support/com.cpa.gui`（auth 檔、config、`usage.db` 都同一份），但**不可同時執行**，兩者會搶 port 8317。

### 10.2 啟用歸檔的三個前提

歸檔要出資料需要**三個條件同時成立**，缺一就是 0 筆：

| # | 條件 | 位置 | 預設 |
| --- | --- | --- | --- |
| 1 | **啟用完整請求歸檔** | 歸檔 DB 的 `archive_metadata` | `false` |
| 2 | **核心 request-log** | GUI `config.toml` → 核心 `config.yaml` | `false` |
| 3 | **關閉 commercial-mode** | GUI `config.toml` → 核心 `config.yaml` | 依安裝而異，**可能是 `true`** |

條件 2 的開關**只存在於本 fork 的歸檔卡片**：upstream 註冊了 `set_core_request_log` 命令，但沒有做任何 UI。

一次驗證三個條件：

```bash
BASE="$HOME/Library/Application Support/com.cpa.gui"
sqlite3 "$BASE/request-records/requests.db" "SELECT key,value FROM archive_metadata;"
grep -n "^request-log\|^commercial-mode" "$BASE/config.toml" "$BASE/cpa-core/config.yaml"
ls "$BASE/oauth/logs" | head        # 應該要有 v1-*.log 之類的逐請求檔，不只 main.log
```

### 10.3 `commercial-mode` 會完全停用 request-log（最容易踩的坑）

`internal/api/server.go:148`：

```go
if !cfg.CommercialMode {
    requestLogger = optionState.requestLoggerFactory(cfg, configFilePath)
    engine.Use(middleware.RequestLoggingMiddleware(requestLogger))
}
```

**商用模式開啟時，請求日誌 middleware 根本不會被註冊。** 後果：

- `request-log: true` 完全無效，且**沒有任何錯誤訊息**
- 設定熱重載會照常記錄 `request-log: false -> true`，但 `requestLogger` 是 `nil`，toggle 什麼都沒做
- **重啟核心也沒用**，只要 commercial-mode 還開著

同一個 gate 也出現在 `internal/runtime/executor/helps/logging_helpers.go:61`（`cfg.RequestLog && !cfg.CommercialMode`）與 `internal/api/server_options.go:50`。`config.go:43` 的註解寫得很清楚：「CommercialMode disables high-overhead request logging」。

**解法**：進階設定關閉商用模式 → **重啟核心**（middleware 在伺服器建構期註冊，熱重載補不回來）。

歸檔卡片已會偵測此狀態：`archiveReadiness()` 回傳 `'commercial-mode'`，顯示紅字說明並把 request-log 開關禁用（避免使用者以為開了就會有效）。
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

## 11. 變更本檔的時機

以下任一情況發生，**必須更新本檔**：

- 新增／移除 fork commit
- 動到 B 區或 C 區任何一行
- 新增對 upstream 的隱性依賴（新的 `invoke` 字串、新的 upstream schema 欄位、新的 upstream CSS class）
- rebase 後 base commit 變更（更新 §1 的 Fork 基底）
- 已知問題被修掉或新增
