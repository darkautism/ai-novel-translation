use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod llm;

use crate::llm::{create_llm_client, LlmClient, LlmConfig};

// --- 結構定義 ---

#[derive(Debug, Deserialize)]
struct Config {
    llm: LlmConfig,
    translation: TranslationConfig,
    constraints: ConstraintsConfig,
    runtime: RuntimeConfig,
}

#[derive(Debug, Deserialize)]
struct TranslationConfig {
    target_language: String,
    input_folder: PathBuf,
    output_folder: PathBuf,
    glossary_folder: PathBuf, // 新增
}

#[derive(Debug, Deserialize)]
struct ConstraintsConfig {
    max_summary_length: usize,
    max_dictionary_size: usize,
}

#[derive(Debug, Deserialize)]
struct RuntimeConfig {
    unattended_mode: bool,
}

// 字典檔案格式
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct ChapterGlossary {
    chapter_name: String,
    summary: String,                // 本章結束後的劇情摘要
    terms: HashMap<String, String>, // 累積到本章為止的所有名詞
}

// Pass 1 AI 回應格式
#[derive(Debug, Deserialize)]
struct AnalysisResponse {
    summary: String,
    new_glossary: HashMap<String, String>,
}

// --- 輔助函式 ---

// 讀取特定章節的字典檔
fn load_glossary(folder: &Path, file_name: &str) -> Option<ChapterGlossary> {
    let path = folder.join(format!("{}.json", file_name));
    if path.exists() {
        let file = fs_err::File::open(path).ok()?;
        serde_json::from_reader(file).ok()
    } else {
        None
    }
}

// 寫入字典檔
fn save_glossary(folder: &Path, file_name: &str, data: &ChapterGlossary) -> Result<()> {
    if !folder.exists() {
        fs_err::create_dir_all(folder)?;
    }
    let path = folder.join(format!("{}.json", file_name));
    let file = fs_err::File::create(path)?;
    serde_json::to_writer_pretty(file, data)?;
    Ok(())
}

// --- 核心處理 ---

async fn process_chapter(
    llm: &dyn LlmClient,
    config: &Config,
    file_path: &Path,
    previous_glossary: &ChapterGlossary,
) -> Result<ChapterGlossary> {
    let file_stem = file_path.file_stem().unwrap().to_string_lossy().to_string();
    let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
    
    println!("正在處理: {}", file_name);
    let content = fs_err::read_to_string(file_path)?;

    // === Pass 1: 分析 (基於上一章的字典與摘要) ===
    println!("  > Pass 1: 分析文本與提取新詞...");
    
    let base_terms_json = serde_json::to_string(&previous_glossary.terms)?;

    let analysis_prompt = format!(
        "你是一個專業的翻譯助手。
        目標：
        1. 目標語言是 {}。
        . 閱讀文章，產生本章節摘要 (最多 {} 字)。
        3. 提取新的專有名詞 (人名、地名、術語) (最多 {} 個)。
        4. 為遵守json格式，摘要請單行且避免使用單雙引號
        
        參考資訊：
        - 上一章摘要: {}
        - 已存在的字典: {} (請勿重複提取已存在的詞，除非需要修正)
        
        請回傳標準 JSON 格式：
        {{
            \"summary\": \"本章摘要...\",
            \"new_glossary\": {{ \"新名詞\": \"中文翻譯\" }}
        }}",
        config.translation.target_language,
        config.constraints.max_summary_length,
        config.constraints.max_dictionary_size,
        previous_glossary.summary,
        base_terms_json
    );

    let raw_resp = llm.generate(&analysis_prompt, &content, false).await?;
    
    // 簡單清理 json block 標記 (防呆)
    let clean_json = raw_resp.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```");

    let analysis: AnalysisResponse = serde_json::from_str(clean_json)
        .context(format!("Pass 1 JSON 解析失敗，原始回應: {}", raw_resp))?;

    // 合併字典：舊字典 + 新字典
    let mut current_terms = previous_glossary.terms.clone();
    current_terms.extend(analysis.new_glossary);

    let current_chapter_data = ChapterGlossary {
        chapter_name: file_stem.clone(),
        summary: analysis.summary,
        terms: current_terms,
    };

    // 立即存檔字典 (這就是你的需求：每一章存一個字典)
    save_glossary(&config.translation.glossary_folder, &file_stem, &current_chapter_data)?;
    println!("    - 字典已存檔至 glossaries/{}.json (目前詞條數: {})", file_stem, current_chapter_data.terms.len());

    // === Pass 2: 翻譯 ===
    println!("  > Pass 2: 翻譯中...");
    
    let final_terms_json = serde_json::to_string(&current_chapter_data.terms)?;
    
    let trans_prompt = format!(
        "你是專業小說翻譯。請將文本翻譯成 {}。
        
        上下文摘要: {}
        
        **嚴格遵守以下名詞對照表**:
        {}

        翻譯後的正文請嚴格遵守以下格式：
        
        章節名稱後空行兩行接著正文，不要包含任何解釋或 markdown 標記，也不要輸出成xml或json。",
        config.translation.target_language,
        current_chapter_data.summary,
        final_terms_json
    );

    let mut translated_text = llm.generate(&trans_prompt, &content, false).await?;

    translated_text = translated_text.replace("\\n", "\n");

    // 寫入翻譯結果
    if !config.translation.output_folder.exists() {
        fs_err::create_dir_all(&config.translation.output_folder)?;
    }

    let output_path = config.translation.output_folder.join(file_name);
    fs_err::write(output_path, translated_text)?;

    Ok(current_chapter_data)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 設定讀取
    // 修正：通常慣例副檔名是 yaml 或 yml，且 Rust crate 主要是 serde_yaml
    let config_path = if Path::new("config.yaml").exists() { "config.yaml" } else { "config.yml" };
    let config_str = fs_err::read_to_string(config_path).context(format!("找不到 {}", config_path))?;
    let config: Config = serde_yml::from_str(&config_str)?;
    
    let llm_client = create_llm_client(&config.llm)?;
    println!("已初始化 LLM Provider: {}", config.llm.provider);



    // 2. 獲取所有輸入檔案並排序
    let mut files: Vec<PathBuf> = WalkDir::new(&config.translation.input_folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    // 檔名自然排序 (這裡簡單用 sort，建議實際專案可引入 alphanumeric-sort)
    files.sort();

    if files.is_empty() {
        println!("輸入資料夾是空的！");
        return Ok(());
    }

    // 3. 自動偵測建議進度 (Auto-Detect Logic)
    let mut suggested_index = 0;
    for (i, file_path) in files.iter().enumerate() {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        let file_stem = file_path.file_stem().unwrap().to_string_lossy();
        
        let output_exists = config.translation.output_folder.join(&*file_name).exists();
        let glossary_exists = config.translation.glossary_folder.join(format!("{}.json", file_stem)).exists();

        // 如果輸出或字典缺一個，就建議從這裡開始
        if !output_exists || !glossary_exists {
            suggested_index = i;
            break;
        }
        // 如果都存在，且是最後一章，建議值會停留在最後一章之後(即 files.len())，但我們會限制它
        if i == files.len() - 1 {
            suggested_index = files.len(); // 代表全部完成
        }
    }

    // 4. 使用者互動與輸入驗證
    println!("=== AI 翻譯工具啟動 ===");
    println!("共發現 {} 個章節檔案。", files.len());
    
    let suggested_display = if suggested_index < files.len() {
        format!("第 {} 章 ({})", suggested_index + 1, files[suggested_index].file_name().unwrap().to_string_lossy())
    } else {
        "全部完成".to_string()
    };
    println!("系統建議從 [{}] 開始。", suggested_display);

    print!("請輸入要開始的章節序號 (1-{}) [按 Enter 使用建議值]: ", files.len());
    io::stdout().flush()?;

    let mut input_buf = String::new();
    io::stdin().read_line(&mut input_buf)?;
    let input = input_buf.trim();

    let start_index = if input.is_empty() {
        if suggested_index >= files.len() {
            println!("根據建議，所有檔案已完成。程式結束。");
            return Ok(());
        }
        suggested_index
    } else {
        match input.parse::<usize>() {
            Ok(n) if n > 0 && n <= files.len() => n - 1, // 轉換為 0-based index
            _ => {
                eprintln!("輸入無效或超出範圍！將強制使用系統建議值: {}", suggested_display);
                if suggested_index >= files.len() { return Ok(()); }
                suggested_index
            }
        }
    };

    println!("-> 已確認從第 {} 章 ({}) 開始執行。", 
        start_index + 1, 
        files[start_index].file_name().unwrap().to_string_lossy()
    );

    // 5. 載入前一章的字典 (Context Loading)
    let mut initial_glossary = ChapterGlossary::default();

    if start_index > 0 {
        let prev_file_stem = files[start_index - 1].file_stem().unwrap().to_string_lossy();
        print!("正在檢查上一章 ({}) 的字典檔... ", prev_file_stem);
        
        if let Some(g) = load_glossary(&config.translation.glossary_folder, &prev_file_stem) {
            println!("成功載入！ (包含 {} 個詞條)", g.terms.len());
            initial_glossary = g;
        } else {
            // 警告邏輯：使用者選了中間章節，但前一章字典不存在
            println!("\n[警告] 找不到上一章的字典檔！");
            println!("這表示 AI 將無法得知之前的劇情摘要與專有名詞，可能會導致翻譯不連貫。");
            print!("確定要使用「空白字典」開始嗎？ (y/N): ");
            io::stdout().flush()?;
            
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm)?;
            if !confirm.trim().eq_ignore_ascii_case("y") {
                println!("使用者取消執行。");
                return Ok(());
            }
            println!("-> 使用空白字典繼續...");
        }
    } else {
        println!("從第一章開始，使用全新字典。");
    }

    // 6. 開始處理迴圈
    let mut current_glossary = initial_glossary;

    for file_path in files.iter().skip(start_index) {
        match process_chapter(&*llm_client, &config, file_path, &current_glossary).await {
            Ok(new_glossary) => {
                current_glossary = new_glossary;
            }
            Err(e) => {
                eprintln!("\n[嚴重錯誤] 處理檔案 {:?} 時失敗: {:?}", file_path, e);
                eprintln!("程式已保留目前進度並停止。修正問題後可再次執行。");
                break;
            }
        }

        // 無人職守控制
        if !config.runtime.unattended_mode {
            print!("\n章節完成。按 Enter 繼續下一章，輸入 'q' 退出: ");
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            if buf.trim().eq_ignore_ascii_case("q") {
                println!("使用者手動停止。");
                break;
            }
        }
    }

    println!("\n工作佇列結束。");
    Ok(())
}