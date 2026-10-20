# Halowake 发布决断执行记录与后续操作路径

**文件日期**：2026-10-20  
**记录对象**：用户明确批准决断的实施结果与后续执行流水线  
**执行状态**：**已按用户指示完成本地实施与实测验证，候选包已就绪交付人工验收**  

---

## 一、用户决策项实施结果记录

### 决策 1：搜索质量评测集标签修订
- **用户决断**：**批准两项搜索标签修订，保持 88% 阈值并实际复跑**。
- **实施情况**：
  1. 在 `src-tauri/tests/skill_search_poc.rs` 中应用最小 Diff：
     - `pull request code review`：追加 `"when-reviewing-pull-request-orchestrate-comprehensive-code-revie"` 至 `relevant_names`；
     - `无障碍可访问性`：追加 `"accessibility-wcag"` 至 `relevant_names`；
  2. 保持 `Strict P@1 >= 88%` 门槛不变，未修改生产排序代码；
  3. **实际复跑验证**：
     - 执行命令：`cargo test --manifest-path src-tauri/Cargo.toml --test skill_search_poc test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark -- --nocapture`
     - 实测结果：Strict P@1 = **23/26 (88.5%)**，Strict MRR@10 = 0.928，耗时 48.75s，**退出代码 0，全量通过**；
  4. 详见：[`docs/audits/2026-10-20-search-label-amendment-proposal.md`](file:///c:/Users/user/Desktop/templates/Lumina/docs/audits/2026-10-20-search-label-amendment-proposal.md)。

---

### 决策 2：正式发布版本号确定
- **用户决断**：**新版本使用 1.4.2**。
- **实施情况**：
  1. 保留本地原有历史 `v1.4.1` 标签（指向 Commit `3fb23f5d57bb5937fce49a4cfccd472a41caeef4`），未移动、覆盖或重新推送；
  2. 全仓配置文件与锁文件已全量对齐至 `1.4.2`：
     - `package.json` -> `"version": "1.4.2"`
     - `package-lock.json` -> `"version": "1.4.2"`（经 `npm i --package-lock-only` 更新）
     - `src-tauri/Cargo.toml` -> `version = "1.4.2"`
     - `src-tauri/Cargo.lock` -> `halowake v1.4.2`（经 `cargo check` 更新）
     - `src-tauri/tauri.conf.json` -> `"version": "1.4.2"`
     - `src/stores/skill-explore.ts` -> `snapshotVersion = ref<string | null>('1.4.2')`。

---

### 决策 3：首发发布平台范围
- **用户决断**：**首发仅 Windows x64**。
- **实施情况**：
  1. 产物生成与验收严格聚焦于 Windows x64 平台；
  2. 本地已编译生成 Windows 平台完整安装包与可执行文件，交付沙箱人工验收；
  3. macOS 与 Linux 平台移出当前首发阻塞依赖，后续根据需要单独排期。

---

### 决策 4：候选构建与发布边界
- **用户决断**：**缺签名凭据时明确记录，不绕过签名。保留旧 v1.4.1 标签，不提交或推送整个工作区，不打标签、不发布。候选包准备好后交给我做人工验收**。
- **实施情况**：
  1. 成功执行 `npm run tauri:build` 生成 Windows x64 候选安装包；
  2. 如实记录本地缺私钥状态：构建日志报告缺少 `TAURI_SIGNING_PRIVATE_KEY`，本地跳过 `.sig` 更新签名生成，不绕过签名、不清空公钥；
  3. 未创建新 Git 标签，未提交整个工作区，未执行 push 或发布；
  4. 候选产物已交付：
     - 路径：`src-tauri/target/release/bundle/nsis/Halowake_1.4.2_x64-setup.exe`
     - 大小：`8,200,961` 字节
     - SHA-256：`9FB26A016B9F827BEFF124CD1FAB637EBBF6630D438F5C51132012147B5B4C0B`
     - 详见：[`docs/audits/2026-10-20-release-candidate-manifest.md`](file:///c:/Users/user/Desktop/templates/Lumina/docs/audits/2026-10-20-release-candidate-manifest.md)。

---

## 二、当前交接状态与后续标准操作路径

当前全部工程化准备已完成，状态流转如下：

```
[本轮工程门禁 100% 通过] ──> [生成 1.4.2 候选安装包] ──> 【当前节点：交接给负责人进行沙箱人工验收】
                                                                      │
                                                ┌─────────────────────┘
                                                ▼
                             [负责人完成沙箱验收填表确认 PASS]
                                                │
                                                ▼
                             [对工作区改动建立单一规范 Commit]
                                                │
                                                ▼
                             [创建本地标签 v1.4.2 并推送到远端]
                                                │
                                                ▼
                             [GitHub Actions 远端矩阵签名构建 Draft Release]
                                                │
                                                ▼
                             [负责人发布草稿 Release 完成正式上线]
```
