# EasyCLIProxyAPI-fork

自行維護的 EasyCLIProxyAPI fork，定期從 upstream 同步後本機建置。

- Upstream：`router-for-me/EasyCLIProxyAPI`
- 維護規範（SOT）：[`FORK.md`](FORK.md)
- 代理協作規範：[`AGENTS.md`](AGENTS.md)

本檔只講「怎麼建、怎麼跑」。改動內容、rebase 對照表、不變式一律以 `FORK.md` 為準。

---

## 這個 fork 多了什麼

**完整請求歸檔（Request Archive）** — 解析核心的 `request-log` transcript，把每筆請求的完整內容存進**獨立的 SQLite 資料庫**，並在「用量統計 → 請求明細」點列查看。

存下來的內容：系統提示詞、訊息、工具定義與呼叫、用量與 token 明細、HTTP 狀態、原始請求與回應報文。

| 項目 | 值 |
| --- | --- |
| 資料庫 | `<core base dir>/request-records/requests.db` |
| 與官方 `usage.db` 的關係 | **完全獨立**，只透過 `request_id` 關聯，官方資料庫一個 byte 都不會被寫入 |
| 原始 log | **永不刪除** |
| 保留策略 | 天數 + 資料庫容量上限，皆可在 UI 調整 |

另外把 app 改名為 `EasyCLIProxyAPI-fork`（視窗標題、bundle identifier `com.cpa.gui.fork`、產出檔名），方便與官方版並存辨識。

---

## 建置

### 一行指令（推薦）

在 **workspace 根目錄**（`EasyCLIProxyAPI` 的上一層）：

```bash
./sync-and-build.sh --check     # 先看 upstream 有沒有更新，只 fetch 不改東西
./sync-and-build.sh --app       # 同步三個 repo + 建置 + 驗證 + 打包 + 產生 .app
```

只改了程式碼、想最快看到結果：

```bash
./sync-and-build.sh --app-only  # 跳過 git 與核心/面板，約 1 分鐘
```

`--check` 的離開碼可供排程使用：`0` = 已是最新，`10` = 有更新待套用。

其他選項見 `./sync-and-build.sh --help`。

### 手動建置

**`.app` bundle（平常用這個）**

```bash
cd EasyCLIProxyAPI
bun install
bun tauri build
open src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

**可攜版**

```bash
cd EasyCLIProxyAPI
./build.sh      # 產出 bin-work/EasyCLIProxyAPI-fork
./run.sh
```

### ⚠️ 兩種產物的資料目錄不同

| 產物 | 資料目錄 | 結果 |
| --- | --- | --- |
| **`.app` bundle** | `~/Library/Application Support/com.cpa.gui` | **與官方版共用**，看得到你的憑證與用量記錄 |
| 可攜版 | 執行檔旁邊的 `bin-work/` | **自帶一份空白 profile**：沒有憑證、沒有用量、歸檔永遠 0 筆 |

這是 upstream 的設計（可攜版本來就該自我包含），不是 bug。**要用真實資料測試就一定要建 `.app`。** 原理見 `FORK.md` §9.1。

兩個版本**不可同時執行**，會搶 port 8317：

```bash
pkill -x cpa-gui; sleep 3
pgrep -fl "cpa-gui|cli-proxy-api" || echo "已全部關閉"
```

---

## 啟用歸檔

需要**三個條件同時成立**，缺一就是 0 筆記錄：

1. **用量統計 → 資料管理 → 完整請求歸檔 → 勾選「啟用完整請求歸檔」**
2. **同一張卡片上開啟「核心 request-log」** — 這是資料來源，預設關閉，而且本 fork 的這張卡片是整個 app 唯一能開它的地方
3. **進階設定關閉「商用模式」並重啟核心** — 商用模式會讓核心完全跳過請求日誌 middleware，`request-log` 開了也沒用，且**不會有任何錯誤訊息**

第 3 點最容易踩，詳見 `FORK.md` §9.3。

驗證三個條件：

```bash
BASE="$HOME/Library/Application Support/com.cpa.gui"
sqlite3 "$BASE/request-records/requests.db" "SELECT key,value FROM archive_metadata;"
grep -n "^request-log\|^commercial-mode" "$BASE/config.toml" "$BASE/cpa-core/config.yaml"
ls "$BASE/oauth/logs" | head        # 要有 v1-*.log 之類的逐請求檔，不只 main.log
```

---

## 磁碟用量（重要）

歸檔開啟後**有兩份資料同時在長**：

| 來源 | 誰負責清理 | 注意 |
| --- | --- | --- |
| 核心的 `logs/*.log` | 核心設定 `logs-max-total-size-mb` | **預設 `0` = 不限制**，會無限成長 |
| `request-records/requests.db` | 歸檔的保留天數 / 容量上限 | 預設 30 天 / 5120 MB |

實測：重度使用（Claude Code）單筆請求的 log 可達 7 MB，歸檔記錄約 4.8 MB。**一小時可用掉數百 MB。**

建議：

- 進階設定把 **`logs-max-total-size-mb` 設成 2048** 之類的值，讓核心自動清理舊檔
- 歸檔的「单字段上限 KB」預設 4096（4 MB）。調高能保留更完整的長對話，但資料庫會等比例膨脹

---

## 直接查資料庫

```bash
DB="$HOME/Library/Application Support/com.cpa.gui/request-records/requests.db"

sqlite3 -header -column "$DB" "
SELECT captured_at, model, http_status, total_tokens, request_id
FROM request_records ORDER BY captured_at_ms DESC LIMIT 20;"

sqlite3 "$DB" "
SELECT system_prompt, messages_json, tools_json, usage_json
FROM request_records WHERE request_id='<id>';"
```

> 歸檔內容包含系統提示詞與完整請求／回應報文，資料庫**未加密**，請自行控管檔案權限。核心只會遮蔽名稱含 `authorization` / `api-key` / `token` / `secret` 的 header，其餘 header 原文寫入。

---

## 出事時

| 狀況 | 動作 |
| --- | --- |
| 歸檔壞掉，但想繼續用 app | `CPA_FORK_ARCHIVE=0 open .../EasyCLIProxyAPI-fork.app` |
| 想完全回到官方版 | `open /Applications/EasyCLIProxyAPI.app` |
| rebase 衝突 | 照 `FORK.md` §3 對照表解，再 `./sync-and-build.sh --no-sync` |
| 建置腳本壞掉 | 走 `FORK.md` §6.3 的手動流程 |

`CPA_FORK_ARCHIVE` 接受的關閉值：`0`、`false`、`off`、`no`（不分大小寫）。關閉後既有的 `requests.db` 會原樣保留。

兩個版本共用同一份 auth 檔、config 與 `usage.db`，切換不會掉資料。
