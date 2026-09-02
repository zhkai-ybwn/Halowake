use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::storage::AppDatabase;

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexReportPromptTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
    pub is_builtin: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub const DEFAULT_STANDARD_REPORT_PROMPT: &str = r#"请将下方 AI 工具（Codex / Claude Code / Antigravity / OpenCode）工作记录整理为一份**简洁、专业、适合日报及绩效评估的中文工作总结**。

要求：

1. **严格基于事实**
   * 不编造未完成事项、业务收益、效率数据、工作时长或个人贡献。
   * 不根据 AI 会话时间推断实际工作量。

2. **只写“今日完成”**
   * 不生成“待跟进”“明日计划”等无事实依据的内容。
   * 只整理已有明确结果的工作事项。

3. **突出成果，不写过程**
   * 优先表达：**完成了什么、解决了什么、覆盖哪些范围、最终结果如何**。
   * 不罗列文件路径、代码行号、命令执行过程等无关细节。

4. **体现实际工作价值**
   * 在事实支持的前提下，优先体现：
     * 前后端、接口、数据库等跨层修改；
     * 查询、列表、详情、搜索等覆盖范围；
     * 多语言、多状态、多场景适配；
     * 兼容性处理；
     * 测试、校验等明确结果。
   * 不夸大，不使用“显著提升”“重大突破”“极大优化”等无事实依据的表述。

5. **合并跨工具重复事项**
   * 按项目归类。
   * 多个工具相同目标下的修改合并成一条成果。
   * 一个会话包含多个独立成果时可以拆分。

6. **尽可能精简**
   * 每个项目优先控制在 **1～3 条**。
   * 每条尽量控制在 **1～2 句话**。
   * 删除重复描述和实现细节。
   * 保留能体现工作范围、技术处理和交付结果的信息。

## 输出格式

# 工作日报｜YYYY-MM-DD

## 今日完成

### 项目名称

1. **成果标题**：简洁描述完成事项、覆盖范围和结果。
2. **成果标题**：……

## 最终要求

* 内容可直接提交日报；
* 专业、客观、有成果感；
* 在不改变事实的前提下，尽可能准确体现实际工作贡献；
* **宁可少写，不要长篇大论。**"#;

pub const DEFAULT_STANDUP_PROMPT: &str = r#"请根据下方 AI 辅助编程工作记录，整理为一份**敏捷开发每日站会 (Daily Standup)** 发言：

## 格式要求：
- **【昨日/今日完成】**：按项目列出核心产出与解决的问题（1-3条，精简有成果感）。
- **【今日计划】**：根据已完成上下文简要列出合理的下一步跟进项。
- **【风险与阻塞】**：无（或如有未解决报错则简要提炼）。"#;

pub const DEFAULT_TECH_SUMMARY_PROMPT: &str = r#"请将下方工作记录整理为一份**技术攻坚与重构小结**，分模块提炼核心技术点、修改范围、架构/逻辑变动及验证结果。语言风格严谨、技术向、要点清晰。"#;

pub fn default_builtin_report_templates() -> Vec<CodexReportPromptTemplate> {
    let now = current_time_ms();
    vec![
        CodexReportPromptTemplate {
            id: "builtin-standard".to_string(),
            name: "标准日报".to_string(),
            content: DEFAULT_STANDARD_REPORT_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 1,
            created_at: now,
            updated_at: now,
        },
        CodexReportPromptTemplate {
            id: "builtin-standup".to_string(),
            name: "敏捷站会".to_string(),
            content: DEFAULT_STANDUP_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 2,
            created_at: now,
            updated_at: now,
        },
        CodexReportPromptTemplate {
            id: "builtin-tech".to_string(),
            name: "技术攻坚".to_string(),
            content: DEFAULT_TECH_SUMMARY_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 3,
            created_at: now,
            updated_at: now,
        },
    ]
}

pub fn list_codex_report_templates(
    database: &AppDatabase,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let connection = database.connect()?;
    let mut stmt = connection
        .prepare(
            "SELECT id, name, content, is_builtin, sort_order, created_at, updated_at
             FROM codex_report_templates
             ORDER BY sort_order ASC, created_at ASC",
        )
        .map_err(|error| format!("查询 Prompt 模板列表失败: {error}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(CodexReportPromptTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                is_builtin: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("遍历 Prompt 模板失败: {error}"))?;

    let mut list = Vec::new();
    for item in rows {
        list.push(item.map_err(|error| format!("解析 Prompt 模板行失败: {error}"))?);
    }

    if list.is_empty() {
        drop(stmt);
        drop(connection);
        let defaults = default_builtin_report_templates();
        for t in &defaults {
            save_codex_report_template(database, t)?;
        }
        return Ok(defaults);
    }

    Ok(list)
}

pub fn save_codex_report_template(
    database: &AppDatabase,
    template: &CodexReportPromptTemplate,
) -> Result<(), String> {
    let connection = database.connect()?;
    let now = current_time_ms();
    let updated_at = if template.updated_at > 0 { template.updated_at } else { now };
    let created_at = if template.created_at > 0 { template.created_at } else { now };

    connection
        .execute(
            "INSERT INTO codex_report_templates (id, name, content, is_builtin, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               content = excluded.content,
               is_builtin = excluded.is_builtin,
               sort_order = excluded.sort_order,
               updated_at = excluded.updated_at",
            params![
                template.id,
                template.name,
                template.content,
                if template.is_builtin { 1 } else { 0 },
                template.sort_order,
                created_at,
                updated_at,
            ],
        )
        .map_err(|error| format!("保存 Prompt 模板失败: {error}"))?;

    Ok(())
}

pub fn delete_codex_report_template(
    database: &AppDatabase,
    id: &str,
) -> Result<(), String> {
    let connection = database.connect()?;

    // 检查是否为内置模板
    let is_builtin: Option<i64> = connection
        .query_row(
            "SELECT is_builtin FROM codex_report_templates WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("查询 Prompt 模板失败: {error}"))?;

    match is_builtin {
        Some(1) => Err("系统内置模板不允许删除".to_string()),
        Some(_) => {
            connection
                .execute("DELETE FROM codex_report_templates WHERE id = ?1", params![id])
                .map_err(|error| format!("删除 Prompt 模板失败: {error}"))?;
            Ok(())
        }
        None => Ok(()),
    }
}

pub fn reset_builtin_codex_report_templates(
    database: &AppDatabase,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let defaults = default_builtin_report_templates();
    for t in &defaults {
        save_codex_report_template(database, t)?;
    }
    list_codex_report_templates(database)
}
