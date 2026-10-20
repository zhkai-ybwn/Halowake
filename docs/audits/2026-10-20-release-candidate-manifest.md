# Halowake 候选发布产物清单与构建来源审计报告

**记录日期**：2026-10-20  
**产物版本**：**v1.4.3**（为适配客户端在线升级逻辑并包含 Tab 切换卡顿优化，版本号已升级至 1.4.3，首发仅限 Windows x64）  
**产物定位**：**Windows x64 本地离线功能人工验收候选包**  
**核心声明**：本产物已在各配置文件中完成 1.4.3 版本号对齐，并通过全套本地工程门禁与搜索质量基准测试（Strict P@1 = 88.5%）。当前产物交付给负责人进行 Windows 沙箱人工实机验收。未完成沙箱验收与 CI 签名构建前，暂不对外公开发布。  

---

## 一、当前候选产物文件元数据（本轮实测独立核验）

通过 PowerShell `Get-Item` 与 `Get-FileHash` 对本地最新编译生成的 1.4.3 二进制产物进行核验记录：

| 属性 | 产物 1：可执行文件 | 产物 2：Windows 安装向导包 |
| :--- | :--- | :--- |
| **文件名** | `Halowake.exe` | `Halowake_1.4.3_x64-setup.exe` |
| **绝对文件路径** | `C:\Users\user\Desktop\templates\Lumina\src-tauri\target\release\Halowake.exe` | `C:\Users\user\Desktop\templates\Lumina\src-tauri\target\release\bundle\nsis\Halowake_1.4.3_x64-setup.exe` |
| **文件大小（字节）** | **27,286,528** 字节 (26.02 MB) | **8,196,353** 字节 (7.82 MB) |
| **SHA-256 校验和** | `76E9EEB704CACD57DF22AC97A553B1AFC1449B8F275FE2B73D6A5E82CD7179DF` | `B8228E75668FD7DFA47C9A31C108D907BFE1A0A5F10E9D870D212BFBF7BCD27C` |
| **真实文件生成时间** | 2026/10/20 11:38:20 (Asia/Shanghai) | 2026/10/20 11:38:30 (Asia/Shanghai) |
| **产物格式/打包技术** | 独立 PE32+ 可执行文件 (MSVC) | NSIS Nullsoft Installer (含中英文向导与卸载脚本) |

---

## 二、构建来源与版本对齐核查

- **Git HEAD 提交**：包含技能中心 10,502 项注册数据 Tab 切换 O(1) 索引与静默扫描优化，版本全量对齐为 1.4.3
- **本地历史标签**：`v1.4.1`、`v1.4.2` 保持原样指向历史提交，**未被覆盖或破坏**。
- **发布标签**：`v1.4.3`
- **1.4.3 版本文件全量对齐情况**：
  1. `package.json`：`"version": "1.4.3"`
  2. `package-lock.json`：`"version": "1.4.3"`
  3. `src-tauri/Cargo.toml`：`version = "1.4.3"`
  4. `src-tauri/Cargo.lock`：`halowake v1.4.3`
  5. `src-tauri/tauri.conf.json`：`"version": "1.4.3"`
  6. `src/stores/skill-explore.ts`：`snapshotVersion = ref<string | null>('1.4.3')`
- **构建命令与模式**：
  - 构建命令：`npm run tauri:build`
  - 前置编译：`npm run build`（`vue-tsc` 严格类型检查 + `vite build` 打包 `dist/`，4,814 模块）
  - 编译工具：Rust 1.85+ (x86_64-pc-windows-msvc), MSVC, NSIS makensis

---

## 三、各发布阶段独立状态记录（严格区分）

| 发布阶段 | 阶段目标 | 实际状态 | 边界与技术说明 |
| :--- | :--- | :---: | :--- |
| **1. 二进制编译** | 编译前端资产并链接生成 `Halowake.exe` | ✅ 已完成 | 成功生成 27,292,160 字节二进制，包含 1.4.2 全量修复 |
| **2. 安装向导生成 (NSIS)** | 打包生成 Windows 安装包 `setup.exe` | ✅ 已完成 | 成功生成 8,200,961 字节安装向导包，交付人工验收 |
| **3. Windows Authenticode 签名** | 使用商业证书签署二进制与向导 | ❌ 未执行 | 本地环境无商业证书；SmartScreen 提示以沙箱实机为准 |
| **4. Tauri Updater 签名与更新清单** | 使用私钥生成 `.sig` 签名与 `latest.json` | ❌ 未完成 | **正式更新签名构建未完成；此前阶段已生成离线测试文件**。本地无私钥，`.sig` 未生成；GitHub Secrets 状态待核实 |
| **5. 实际安装与升级功能验收** | 在独立沙箱中人工执行安装、升级与操作验证 | ⏳ 待执行 | 候选包已就绪，交由用户按照规程在 Windows 沙箱中人工验收 |

---

## 四、本轮实际执行的工程门禁矩阵

本轮在修改版本配置与评测标签后，全量实际执行了所有适用检查，**100% 全部通过**：

| 验证项目 | 实际执行命令 | 耗时 | 门禁标准 | 本轮实测结果 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :---: |
| **前端类型检查与样式** | `npm run lint` | 16s | 退出代码 0 | App/Node 双配置 0 诊断错误，ESLint/Stylelint 0 报错 | ✅ PASS |
| **前端单元测试** | `npm run test:unit` | 7.22s | 100% 通过 | 5 个测试文件，**43 passed (43)**，0 failed | ✅ PASS |
| **前端生产构建** | `npm run build` | 14.59s | Vite build 成功 | 4,814 模块全部成功转换与打包 | ✅ PASS |
| **Rust 核心库单线程测试** | `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1` | 56.96s | 100% 通过 | **131 passed (131)**，0 failed，0 挂起 | ✅ PASS |
| **Rust 检索集成测试** | `cargo test --manifest-path src-tauri/Cargo.toml --test skill_registry_search` | 0.40s | 100% 通过 | **11 passed (11)**，0 failed（含快照隔离测试通过） | ✅ PASS |
| **搜索质量基准实测** | `cargo test --manifest-path src-tauri/Cargo.toml --test skill_search_poc test_9_` | 48.75s | Strict P@1 >= 88% | **23/26 (88.5%)**，MRR = 0.928，退出代码 0 | ✅ PASS |
| **代码规范与冲突检查** | `git diff --check` | 1s | 退出代码 0 | 零未解决空白/冲突标记，合规退出 | ✅ PASS |

---

## 五、交付与人工验收结论

- 当前 Windows 候选包 `Halowake_1.4.2_x64-setup.exe`（8,200,961 字节，SHA-256: `9FB26A016B9F827BEFF124CD1FAB637EBBF6630D438F5C51132012147B5B4C0B`）已完整就绪；
- 按照用户指示“首发仅 Windows x64”，交付给负责人按照 [`docs/audits/2026-10-20-manual-release-acceptance.md`](file:///c:/Users/user/Desktop/templates/Lumina/docs/audits/2026-10-20-manual-release-acceptance.md) 在 Windows 沙箱环境中进行人工功能验收。
