# Halowake 技能搜索评测基准标签修订提案与实测结果

**提案日期**：2026-10-20  
**提案对象**：`src-tauri/tests/skill_search_poc.rs` 中 30 条评测集（`test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark`）  
**审批状态**：**✅ 用户已正式批准（已应用最小修改并实测通过）**  
**门槛标准**：Strict P@1 >= 88%（硬性红线保持不变，未降低门槛，未在生产代码中硬编码特殊判断）  
**历史基线数据**：Strict P@1 = **21/26 (80.8%)**，Strict MRR@10 = **0.870**（退出 101）  
**本轮实测通过数据**：Strict P@1 = **23/26 (88.5%)**，Strict MRR@10 = **0.928**（耗时 48.75s，**退出代码 0，全量通过**）  

---

## 一、评测集性质与背景说明

1. **评测集性质声明**：
   该评测集最初作为独立留出集设计，但在前序多轮迭代中已作为检索调优（如 FTS5 权重、分词、同义词扩展和中英混合处理）的反馈依据。**客观确认该评测集已深度参与开发过程，不应再无条件称其为“完全独立的未知盲测集”**。
2. **治理原则**：
   - 保持 Strict P@1 >= 88% 的硬性红线不降低；
   - 严禁在生产检索或排序逻辑中针对评测 Query 进行硬编码作弊；
   - 严禁为单纯凑足通过率（23/26）而主观放宽判定标准；
   - 严格依据技能底层 Payload 的真实功能定义、用途约束和适用范围，建立统一客观的三级相关性裁决准则。

---

## 二、统一相关性判定准则

| 等级 | 判定分类 | 判定标准 | 示例 |
| :--- | :--- | :--- | :--- |
| **Grade 2** | **完全相关 (Strict Exact)** | 技能核心技术功能、面向场景与查询意图完全契合，且**无特定排他性品牌或专有外部厂商约束**；能直接且权威地解决用户查询的技术诉求。 | 查询“无障碍可访问性”时返回专门覆盖 WCAG 2.2、ARIA 与屏幕阅读器的标准技能。 |
| **Grade 1** | **通用/平台可接受 (Acceptable)** | 技能技术方向与查询意图重叠，但属于通用降级（缺少专有技术栈时的泛化技能），或属于**特定云厂商/托管平台的专有实现**（具备参考价值但非通用标准）。 | 查询“PostgreSQL 数据库优化”时返回 Azure 托管 PostgreSQL 的调优技能；或缺少专属技术栈时返回通用代码审查技能。 |
| **Grade 0** | **不相关 (Irrelevant)** | 技能被词法高频词偶然召回，但其**真实用途被作者严格限制于私有品牌**，或仅为**第三方孤立业务 API 的特定集成**，无法推广为通用开发能力；或与查询完全脱节。 | 查询“frontend design system”时返回明确声明“仅限 YUV.AI 品牌输出”的私有设计系统。 |

---

## 三、争议查询逐项证据与裁决结果

从本机真实数据库（`%APPDATA%/com.yubei.lumina/lumina.db` · 3,176 条技能真实副本）中完整提取 5 项争议条目的底层 Payload，依据准则进行事实核验：

### 3.1 获准修订项 1：`无障碍可访问性` (Grade 2 · 完全相关)
- **查询词 (Query)**：`无障碍可访问性`（所属分类：7.同义词）
- **生产检索实际返回 Top-1**：`accessibility-wcag`
- **真实底层证据**：
  - **ID**: `rohitg00-awesome-claude-code-toolkit-skills-accessibility-wcag`
  - **描述**: *"Web accessibility patterns for WCAG 2.2 compliance including ARIA, keyboard navigation, screen readers, and testing"*
  - **子路径**: `skills/accessibility-wcag` · **星数**: 2,655
- **修订裁决**：**已批准加入白名单**。内容专注 Web 无障碍标准（WCAG 2.2）、ARIA 规范、键盘无障碍导航及屏幕阅读器测试，技术实质完全匹配。

### 3.2 获准修订项 2：`pull request code review` (Grade 2 · 完全相关)
- **查询词 (Query)**：`pull request code review`（所属分类：3.纯英文）
- **生产检索实际返回 Top-1**：`when-reviewing-pull-request-orchestrate-comprehensive-code-revie`
- **真实底层证据**：
  - **ID**: `corpus-aiskillstore-marketplace-skills-dnyoussef-when-reviewing-pull-request-orchestrat-0ecfa0ed`
  - **描述**: *"Use when conducting comprehensive code review for pull requests across multiple quality dimensions. Orchestrates 12-15 specialized reviewer agents across 4 phases using star topology coordination..."*
  - **子路径**: `skills/dnyoussef/when-reviewing-pull-request-orchestrate-comprehensive-code-review`
- **修订裁决**：**已批准加入白名单**。描述与设计完全用于 PR 代码审查编排，技术实质完全匹配。

### 3.3 维持不修订项 1：`frontend design system` (Grade 0 · 不相关)
- **生产返回 Top-1**：`yuv-design-system`
- **描述证据**：*"Yuval Avidani's YUV.AI brand and design system. Apply ONLY when YUV.AI-branded output is requested... Do NOT auto-trigger on generic 'build a game / web app / dashboard / landing page' without YUV.AI context..."*
- **裁决**：**维持不相关判定，严禁列入白名单**。作者已显式排除通用场景，失分属真实检索排序缺陷，后续通过专有品牌负向惩罚机制优化。

### 3.4 维持不修订项 2：`OAuth 登录鉴权` (Grade 0 · 不相关)
- **生产返回 Top-1**：`zoom-oauth`
- **描述证据**：*"Reference skill for Zoom authentication. Use after routing to an auth workflow when choosing app credentials, grant types, scopes..."*
- **裁决**：**维持不相关判定**。Zoom 专属 API 插件，无法推广为通用应用 OAuth 鉴权。

### 3.5 审慎保持项 3：`PostgreSQL 数据库优化` (Grade 1 · 平台专属)
- **生产返回 Top-1**：`azure-database-postgresql`
- **裁决**：**维持暂不列入 Strict P@1 相关项**。偏向特定 Azure 托管云基础设施，待后续建立平台通用规则后再行审议。

---

## 四、实际应用的代码补丁与实测通过记录

用户批准后，已在 `src-tauri/tests/skill_search_poc.rs` 中应用最小 Diff：

```diff
--- a/src-tauri/tests/skill_search_poc.rs
+++ b/src-tauri/tests/skill_search_poc.rs
@@ -3154,7 +3154,7 @@ fn test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark() {
         EvalQuery {
             category: "3.纯英文",
             query: "pull request code review",
-            relevant_names: &["review-pr", "code-review", "code-review-and-quality", "autoreview", "requesting-code-review", "receiving-code-review"],
+            relevant_names: &["review-pr", "code-review", "code-review-and-quality", "autoreview", "requesting-code-review", "receiving-code-review", "when-reviewing-pull-request-orchestrate-comprehensive-code-revie"],
             acceptable_names: &["understand-diff", "gh-stack"],
             is_negative: false,
         },
@@ -3242,7 +3242,7 @@ fn test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark() {
         EvalQuery {
             category: "7.同义词",
             query: "无障碍可访问性",
-            relevant_names: &["accessible-authentication", "web-design-guidelines", "frontend-design", "ui-ux-pro-max"],
+            relevant_names: &["accessible-authentication", "web-design-guidelines", "frontend-design", "ui-ux-pro-max", "accessibility-wcag"],
             acceptable_names: &["super-ui-ux-design", "design-systems"],
             is_negative: false,
         },
```

### 实测验证输出记录：
- 执行命令：`cargo test --manifest-path src-tauri/Cargo.toml --test skill_search_poc test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark -- --nocapture`
- 黄金回归集（30 条）：Strict P@1 = **26/26 (100.0%)**，General Fallback = 2/2，Negative = 2/2
- 独立评测集（30 条）：Strict P@1 = **23/26 (88.5%)**，Strict P@3 = 56.4%，Strict MRR@10 = 0.928
- **测试结果**：`test test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark ... ok`
- **耗时**：48.75s
- **退出代码**：**0（全量通过）**
