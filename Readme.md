# AI Novel Translation (AI 小說翻譯工具)

![CI](https://github.com/darkautism/ai-novel-translation/actions/workflows/ci.yml/badge.svg)

這是一個基於 Rust 開發的 AI 小說翻譯工具，專為翻譯連載小說設計。它採用雙階段處理流程（Two-Pass），確保翻譯的一致性與連貫性。

## 特色

*   **雙階段翻譯 (Two-Pass Translation)**：
    *   **第一階段 (Analysis)**：閱讀文本，產生劇情摘要，並提取新的人名、地名、術語，建立/更新字典。
    *   **第二階段 (Translation)**：基於摘要與累積的字典進行翻譯，確保前後文連貫且專有名詞統一。
*   **上下文感知 (Context Aware)**：每一章的翻譯都會參考上一章的摘要與累積至今的字典。
*   **斷點續傳**：程式會自動偵測進度，若中斷可隨時接續執行，無需從頭開始。
*   **高度客製化**：
    *   支援 **Gemini** 與 **Ollama** (Llama 3, Mistral, Qwen 等) 模型。
    *   **Prompt 模板化**：可在 `config.yml` 中使用模板引擎自定義 AI 指令。
    *   **自動建立資料夾**：初次使用自動建立所需目錄。

## 安裝與編譯

本專案使用 Rust 語言開發。請先確保您的系統已安裝 Rust 工具鏈 (Cargo)。

1.  **複製專案**
    ```bash
    git clone https://github.com/your-repo/ai-novel-translation.git
    cd ai-novel-translation
    ```

2.  **編譯**
    ```bash
    cargo build --release
    ```
    編譯完成後的執行檔位於 `target/release/ai-novel-translation` (Windows 為 `.exe`)。

## 設定說明 (`config.yml`)

專案根目錄下的 `config.yml` 是核心設定檔。程式啟動時會讀取此檔案。

### 1. LLM 設定 (`llm`)
選擇你的 AI 提供者。

```yaml
llm:
  provider: "gemini" # 或 "ollama"

  gemini:
    api_key: "你的_GOOGLE_API_KEY"
    model: "gemini-2.0-flash"

  ollama:
    base_url: "http://localhost:11434"
    model: "llama3:latest"
```

### 2. 翻譯路徑 (`translation`)
設定輸入與輸出的位置。

```yaml
translation:
  target_language: "Traditional Chinese (Taiwan)" # 目標語言
  input_folder: "./input_chapters"   # 放置原文 txt 的資料夾
  output_folder: "./output_chapters" # 翻譯結果存放處
  glossary_folder: "./glossaries"    # 字典檔存放處
```

### 3. Prompt 模板 (`prompts`)
本工具引入了模板引擎，您可以在此完全控制 AI 的行為。使用 `{{ 變數名 }}` 來插入動態內容。

#### 分析階段 (`analysis_prompt`)
此階段目標是生成摘要與提取術語。
*   `target_lang`: 目標語言
*   `summary_len`: 摘要最大長度限制
*   `glossary_limit`: 每次提取新詞數量上限
*   `prev_summary`: 上一章的劇情摘要
*   `existing_glossary`: 目前已存在的字典內容 (JSON 格式)

#### 翻譯階段 (`translation_prompt`)
此階段目標是進行正式翻譯。
*   `target_lang`: 目標語言
*   `summary`: 本章節的劇情摘要
*   `glossary`: 完整的專有名詞對照表 (JSON 格式)

## 使用方法

1.  **準備檔案**：
    *   將小說章節存為 `.txt` 檔案（建議檔名包含序號，如 `001.txt`, `002.txt` 以確保排序正確）。
    *   放入 `input_chapters` 資料夾（若資料夾不存在，執行程式後會自動建立）。

2.  **執行程式**：
    ```bash
    ./target/release/ai-novel-translation
    ```
    *   程式會自動列出所有檔案。
    *   自動偵測建議的開始章節（例如上次翻到第 5 章，這次會建議從第 6 章開始）。

3.  **人工修訂 (可選)**：
    *   `glossaries` 資料夾中會生成對應每一章的 `.json` 字典檔。
    *   **技巧**：如果你發現 AI 提取的名詞有誤，可以手動編輯 `.json` 檔案。下一章翻譯時會自動引用修正後的內容。

## 常見問題

*   **Q: 程式執行後直接結束？**
    *   A: 請檢查 `input_chapters` 是否有 `.txt` 檔案。如果是初次執行，程式建立空資料夾後會提示您放入檔案。
*   **Q: 翻譯品質不佳？**
    *   A: 試著調整 `config.yml` 中的 Prompt。例如強調「不要翻譯人名」或「使用武俠小說風格」等。
*   **Q: 出現 JSON 解析錯誤？**
    *   A: 有時 AI 會輸出不標準的格式。通常重試一次即可。若頻繁發生，可嘗試更換更聰明的模型 (如 Gemini 1.5 Pro / GPT-4 等級)。

## License

MIT License
