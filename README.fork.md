# EasyCLIProxyAPI-fork

自行維護的 EasyCLIProxyAPI fork，定期從 upstream 同步後本機建置。

- Upstream：`router-for-me/EasyCLIProxyAPI`
- 維護規範（SOT）：[`FORK.md`](FORK.md)
- 代理協作規範：[`AGENTS.md`](AGENTS.md)
- 可重複執行的同步、文件更新與 release runbook：[`SYNC-UPDATE-RELEASE.md`](SYNC-UPDATE-RELEASE.md)

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
export RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin
export TMPDIR=/private/tmp

./sync-and-build.sh --check     # 先看 upstream 有沒有更新，只 fetch 不改東西
./sync-and-build.sh --app       # 同步三個 repo + 建置 + 驗證 + 打包 + 產生 .app
```

目前 macOS Rust suite 會在一個已知的 symlink cleanup case 失敗。完成 release build 時，先跑上面的驗證，然後改用：

```bash
./sync-and-build.sh --no-sync --app --no-verify
```

只改了程式碼、想最快看到結果：

```bash
./sync-and-build.sh --app-only --no-verify  # 跳過 git 與核心/面板，產生 .app 與 DMG
```

此 Apple Silicon macOS 工作站的 release build 必須使用 Rust 1.90.0。Rust 1.98.1 編譯 `cpa-gui` 時會發生 `SIGBUS`，即使 `cargo clean` 後仍會重現；在該編譯器修正前不可用於 portable、`.app` 或 DMG 建置。

Rust test 在 macOS 另有環境限制：預設的 `/var/folders/...` 暫存目錄會因 `/var` 是 symlink 而使 66 個 `agents::backups` 測試失敗。請改用：

```bash
cd EasyCLIProxyAPI/src-tauri
TMPDIR=/private/tmp RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin cargo test
```

此設定下目前僅剩 `linked_configuration_and_backup_directories_are_rejected` 一個已知的 symlink cleanup 測試失敗（詳見 `FORK.md` §8）；它不影響 release build。若 `--app` 已完成同步但停在這個測試，請用 `TMPDIR=/private/tmp RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin ./sync-and-build.sh --no-sync --app --no-verify` 完成 portable、`.app` 與 DMG。

`--check` 的離開碼可供排程使用：`0` = 已是最新，`10` = 有更新待套用。

其他選項見 `./sync-and-build.sh --help`。

### 手動建置

**`.app` bundle（平常用這個）**

```bash
cd EasyCLIProxyAPI
export RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin
bun install
bun tauri build
open src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

**可攜版**

```bash
cd EasyCLIProxyAPI
export RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin
./build.sh      # 產出 bin-work/EasyCLIProxyAPI-fork
./run.sh
```

### ⚠️ 兩種產物的資料目錄不同

| 產物 | 資料目錄 | 結果 |
| --- | --- | --- |
| **`.app` bundle** | `~/Library/Application Support/com.cpa.gui` | **與官方版共用**，看得到你的憑證與用量記錄 |
| 可攜版 | 執行檔旁邊的 `bin-work/` | **自帶一份空白 profile**：沒有憑證、沒有用量、歸檔永遠 0 筆 |

這是 upstream 的設計（可攜版本來就該自我包含），不是 bug。**要用真實資料測試就一定要建 `.app`。** 原理見 `FORK.md` §10.1。

兩個版本**不可同時執行**，會搶 port 8317：

```bash
pkill -x cpa-gui; sleep 3
pgrep -fl "cpa-gui|cli-proxy-api" || echo "已全部關閉"
```

---

## 啟用歸檔

需要**三個條件同時成立**，缺一就是 0 筆記錄：

1. **用量統計 → 資料管理 → 完整請求歸檔 → 勾選「啟用完整請求歸檔」後按「儲存設定」**
2. **同一張卡片上開啟「核心 request-log」** — 這是資料來源，預設關閉，而且本 fork 的這張卡片是整個 app 唯一能開它的地方
3. **進階設定關閉「商用模式」並重啟核心** — 商用模式會讓核心完全跳過請求日誌 middleware，`request-log` 開了也沒用，且**不會有任何錯誤訊息**

第 3 點最容易踩，詳見 `FORK.md` §10.3。

「啟用完整請求歸檔」、保留天數、資料庫上限、日誌目錄上限與單欄位上限共用一份草稿。可直接輸入數字或用上下按鈕調整，只有按「儲存設定」才會寫入。儲存後卡片會重新讀取後端設定，所有欄位都與送出值一致才顯示成功通知。核心 `request-log` 是另一個即時開關，不受這個按鈕影響。

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

## 版本號與更新

fork 的版本號**繼承自 upstream**（rebase 時自動帶入），用來標示目前的 upstream 基底，不需要手動維護。

**app 內的自動更新已停用。** 原因：app 啟動時會檢查 upstream 的 releases，而 fork 繼承了 upstream 的版本號，所以 upstream 一發新版就會顯示「有更新」，按下去會**用官方版覆蓋掉 fork 的所有改動**。

版本管理頁會顯示停用原因並指向重建指令。要升級就跑：

```bash
./sync-and-build.sh --app
```

核心（CLIProxyAPI）的更新不受影響，仍照原本方式運作。

---

## 維運指令

以下全部以這個變數為前提：

```bash
BASE="$HOME/Library/Application Support/com.cpa.gui"
DB="$BASE/request-records/requests.db"
```

### 健康檢查（先跑這個）

一次看完所有關鍵狀態：

```bash
echo "— 執行中的程序 —"; pgrep -fl "cpa-gui|cli-proxy-api" || echo "(未執行)"
echo "— 三個開關 —"
sqlite3 "$DB" "SELECT key,value FROM archive_metadata;" 2>/dev/null
grep -n "^request-log\|^commercial-mode\|^logs-max-total-size-mb" "$BASE/config.toml"
echo "— 磁碟 —"
du -sh "$BASE/oauth/logs"; ls -lh "$DB" | awk '{print "requests.db:", $5}'
sqlite3 "$DB" "SELECT COUNT(*) FROM request_records;" | xargs echo "歸檔筆數:"
```

判讀：`enabled=true` + `request-log = true` + `commercial-mode = false`，三者缺一就不會有記錄。

### 歸檔沒有資料時的排查順序

```bash
# 1. 來源檔存在嗎？只有 main.log 代表核心沒在寫 transcript
ls -t "$BASE/oauth/logs" | head -5

# 2. 核心是否確實載入了設定（找設定重載事件）
grep -n "request-log:" "$BASE/oauth/logs/main.log" | tail -3

# 3. 開啟後有沒有新請求（management 呼叫不算）
grep -E "POST|GET" "$BASE/oauth/logs/main.log" | grep -v "v0/management" | tail -5
```

若第 1 步只有 `main.log`，八成是 `commercial-mode` 還開著 —— 那會讓核心**完全跳過**請求日誌 middleware，`request-log` 開了也沒用，而且不會有任何錯誤訊息。詳見 `FORK.md` §10.3。

### 檢視歸檔內容

```bash
# 最近 20 筆
sqlite3 -header -column "$DB" "
SELECT captured_at, model, http_status, total_tokens, request_id
FROM request_records ORDER BY captured_at_ms DESC LIMIT 20;"

# 各欄位是否有值（0 代表解析失敗，要注意）
sqlite3 -header -column "$DB" "
SELECT model, http_status, total_tokens,
       LENGTH(system_prompt) sys_len, LENGTH(request_body) body_len,
       LENGTH(tools_json) tool_len, LENGTH(usage_json) usage_len, truncated
FROM request_records ORDER BY captured_at_ms DESC LIMIT 10;"

# 單筆完整內容
sqlite3 "$DB" "
SELECT system_prompt, tools_json, usage_json, request_body
FROM request_records WHERE request_id='<id>';"
```

> 訊息不再獨立儲存，改由 `request_body` 即時解析，所以查對話內容要看 `request_body`。

### 驗證 token 數是否正確

與核心自己的用量資料庫交叉比對。兩者是**完全獨立的來源**（我們解析 log 檔，核心走 usage 頻道），數字一致就代表解析正確：

```bash
U="$BASE/usage-records/usage.db"
for rid in $(sqlite3 "$DB" "SELECT request_id FROM request_records WHERE http_status=200 ORDER BY captured_at_ms DESC LIMIT 3;"); do
  echo "--- $rid ---"
  sqlite3 -header -column "$DB" "SELECT 'archive' src, input_tokens, output_tokens, cache_read_tokens, total_tokens FROM request_records WHERE request_id='$rid';"
  sqlite3 -header -column "$U"  "SELECT 'usage  ' src, input_tokens, output_tokens, cache_read_tokens, total_tokens FROM usage_events WHERE request_id='$rid';"
done
```

### 磁碟用量分析

```bash
# 哪個欄位在吃空間（MB）
sqlite3 -header -column "$DB" "
SELECT SUM(LENGTH(request_body))/1048576   AS request_body,
       SUM(LENGTH(response_body))/1048576  AS response_body,
       SUM(LENGTH(system_prompt))/1048576  AS system_prompt,
       SUM(LENGTH(tools_json))/1048576     AS tools_json,
       SUM(LENGTH(messages_json))/1048576  AS messages_json_legacy
FROM request_records;"

# 平均每筆多大
sqlite3 "$DB" "SELECT COUNT(*) || ' 筆, 平均 ' || (SUM(byte_size)/COUNT(*)/1048576) || ' MB/筆' FROM request_records;"

# 最大的來源 log
ls -lhS "$BASE/oauth/logs" | head -5
```

參考值：改用即時解析後約 **3.5 MB/筆**（先前重複儲存訊息時是 6.1 MB）。

### 清理（UI 優先，不用關 app）

歸檔卡片下方有三顆按鈕，**這是首選做法**，app 執行中就能操作：

| 按鈕 | 作用 | 安全性 |
| --- | --- | --- |
| **压缩数据库** | WAL checkpoint + `VACUUM`，回收已刪除列佔用的空間 | 不影響資料 |
| **清理已归档日志** | 刪除**已經歸檔**的 `v1-*.log`，以及 30 分鐘前的殘留暫存目錄 | **未歸檔的檔案會保留並計入「跳過」** |
| **清空归档** | 清空 `request_records` 並 `VACUUM` | 設定保留、原始 log 不動 |

完成後會顯示「清理 N 項，釋放 X，跳過未歸檔 M 項」。

「清理已归档日志」永不觸碰 `main.log` 與 `response-body-*`，判定規則與掃描器**共用同一個函式**，不會出現「掃描不認、清理卻刪」的矛盾。

### 清理（命令列，UI 壞掉時的後備）

**務必先關掉 app**，否則會與執行中的連線衝突：

```bash
pkill -x cpa-gui; sleep 3
pgrep -fl "cpa-gui|cli-proxy-api" || echo "已全部關閉"
```

```bash
# 清空歸檔（設定會保留，原始 log 不受影響）
sqlite3 "$DB" "DELETE FROM request_records;" && sqlite3 "$DB" "VACUUM;"

# 清掉舊的來源 log
find "$BASE/oauth/logs" \( -name "v1-*.log" -o -name "v1beta-*.log" -o -name "request-log-parts-*" \) -delete
```

> ⚠️ **只清歸檔而不清 log，重啟後會全部長回來** —— 去重鍵是 `source_file`，log 檔還在就會被重新歸檔。要真的縮小就兩個都清。

**刪除列不會讓檔案變小。** SQLite 把釋出的頁面掛在 free list 上，檔案維持在歷史高點，必須 `VACUUM` 重建才會縮小：

```bash
sqlite3 "$DB" "VACUUM;"     # 或按 UI 的「压缩数据库」
```

清空歸檔後看到「0 筆記錄」卻仍佔用數百 MB，就是這個原因。

### 清掉歸檔後舊請求點不開是正常的

事件列表讀 `usage-records/usage.db`（核心的用量頻道），歸檔讀 `request-records/requests.db`。**兩者完全獨立**，所以清掉歸檔後，事件列表照樣列出 1400 筆舊請求，但點進去會顯示找不到歸檔內容。

歸檔的實際涵蓋範圍：

```bash
sqlite3 -header -column "$DB" "SELECT COUNT(*) n, MIN(captured_at) oldest, MAX(captured_at) newest FROM request_records;"
```

### 重新歸檔（改動解析邏輯後套用到舊資料）

歸檔的去重鍵是 `source_file`，指紋相同就不會重讀。修正解析邏輯後要讓舊記錄套用新版，先刪掉那些記錄，採集器會在下個 5 秒週期用新邏輯重新讀取來源 log。

```bash
# 例：訊息為空且被截斷的記錄（早期版本從截斷的 body 解析失敗）
sqlite3 "$DB" "DELETE FROM request_records WHERE messages_json='' AND truncated=1;"

# 例：特定模型全部重來
sqlite3 "$DB" "DELETE FROM request_records WHERE model='claude-opus-5';"
```

**前提是來源 log 還在。** 已被清理掉的 log 無法重新歸檔，那些記錄刪掉就永久消失。先確認：

```bash
sqlite3 "$DB" "SELECT COUNT(*) FROM request_records WHERE messages_json='' AND truncated=1;"
ls "$BASE/oauth/logs"/*.log | wc -l
```

### 控制成長速度

兩個獨立的成長源，要分開設定：

| 設定 | 位置 | 作用 |
| --- | --- | --- |
| 日誌檔案總容量上限 | 進階設定 | 核心自己的上限，**只算頂層 `.log`，不含 spill 子目錄**；預設 `0` = 不限制 |
| **日誌目錄上限 MB** | **歸檔卡片** | **fork 自己的上限，遞迴計算含子目錄，只刪已歸檔的檔案；`0` = 交給核心** |
| 資料庫上限 MB | 歸檔卡片 | `requests.db` 的上限 |
| 保留天數 | 歸檔卡片 | 依時間清理 |
| 单字段上限 KB | 歸檔卡片 | 單一欄位截斷點，直接決定每筆多大 |

表中的歸檔卡片設定共用「儲存設定」按鈕，調整數字不會立即套用。

核心的清理器要**重啟核心**才會套用新的上限。

### 設定備份與還原

改設定前先備份：

```bash
cp "$BASE/config.toml" "$BASE/config.toml.bak.$(date +%Y%m%d-%H%M%S)"
```

`config.toml` 是 GUI 層的來源，核心的 `cpa-core/config.yaml` 會在每次啟動時由它覆寫，所以**只改 yaml 不會持久**。

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
