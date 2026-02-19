# AI Novel Translation

![CI](https://github.com/darkautism/ai-novel-translation/actions/workflows/ci.yml/badge.svg)

AI Novel Translation is a Rust-based tool for translating serialized novels with stronger consistency between chapters.  
It uses a two-pass workflow to build context first, then generate the final translation.

## Features

- **Two-Pass Translation**
  - **Pass 1 (Analysis):** Reads the chapter, generates a summary, and extracts new proper nouns/terms.
  - **Pass 2 (Translation):** Translates the chapter using the summary and cumulative glossary.
- **Context-Aware Pipeline:** Each chapter uses the previous chapter summary and accumulated glossary.
- **Resume Support:** The tool auto-detects progress so interrupted jobs can continue from the suggested chapter.
- **Highly Configurable**
  - Supports **Gemini**, **Ollama** (Llama 3, Mistral, Qwen, etc.), and **OpenAI-compatible** providers.
  - Prompt templates are fully configurable in `config.yml`.
  - Required folders are created automatically on first run.

## Build

This project is written in Rust. Make sure Rust/Cargo is installed first.

1. **Clone**
   ```bash
   git clone https://github.com/your-repo/ai-novel-translation.git
   cd ai-novel-translation
   ```

2. **Build**
   ```bash
   cargo build --release
   ```
   The binary is generated at `target/release/ai-novel-translation` (`.exe` on Windows).

## Configuration (`config.yml`)

`config.yml` in the project root is the main runtime configuration file.

### 1) LLM Settings (`llm`)

Choose your provider:

```yaml
llm:
  provider: "gemini" # or "ollama" / "openai"

  gemini:
    api_key: "YOUR_GOOGLE_API_KEY"
    model: "gemini-2.0-flash"

  ollama:
    base_url: "http://localhost:11434"
    model: "llama3:latest"

  openai:
    api_key: "YOUR_OPENAI_KEY"
    model: "gpt-4o"
```

### 2) Translation Paths (`translation`)

Define input/output locations:

```yaml
translation:
  target_language: "Traditional Chinese (Taiwan)"
  input_folder: "./input_chapters"
  output_folder: "./output_chapters"
  glossary_folder: "./glossaries"
```

### 3) Prompt Templates (`prompts`)

Templates use `{{ variable_name }}` syntax.

#### Analysis Prompt (`analysis_prompt`)

Goal: generate chapter summary and extract new terms.

- `target_lang`: target language
- `summary_len`: max summary length
- `glossary_limit`: max number of extracted terms
- `prev_summary`: previous chapter summary
- `existing_glossary`: current glossary content (JSON string)

#### Translation Prompt (`translation_prompt`)

Goal: generate final translation text.

- `target_lang`: target language
- `summary`: current chapter summary
- `glossary`: full glossary mapping (JSON string)

## Usage

1. **Prepare input files**
   - Save chapters as `.txt` files (recommended naming: `001.txt`, `002.txt`, ... for stable order).
   - Put them into `input_chapters` (created automatically if missing).

2. **Run**
   ```bash
   ./target/release/ai-novel-translation
   ```
   - The tool scans chapter files and suggests a start point based on existing outputs/glossaries.
   - You can press Enter to use the suggestion or manually select a chapter number.

3. **Optional manual glossary edits**
   - Each chapter produces a glossary JSON in `glossaries/`.
   - You can manually fix term mappings; later chapters will use your edits automatically.

## FAQ

- **Q: The program exits immediately after start.**
  - A: Check whether `input_chapters` contains `.txt` files. On first run, the tool may only create folders and exit.
- **Q: Translation quality is not ideal.**
  - A: Tune the prompts in `config.yml` (style, constraints, naming rules, etc.).
- **Q: I got a JSON parse error.**
  - A: Some model outputs can still be malformed occasionally. Retry first, then try a stronger model if needed.

## License

MIT License
