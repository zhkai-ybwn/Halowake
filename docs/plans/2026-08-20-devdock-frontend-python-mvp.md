# DevDock 前端与 Python 项目 MVP 设计及实施计划

> 日期：2026-08-20  
> 状态：设计基线，待按阶段实施  
> 范围：前端项目、Python 项目、命令配置、命令执行、进程管理、日志查看  
> 明确延期：Java、Docker、Go、Rust、工作区递归扫描、远程进程

## 1. 目标

DevDock 第一阶段要完成一条稳定、容易理解的主流程：

```text
添加项目
  → 识别项目类型
  → 扫描可运行命令
  → 用户直接运行或接受候选命令
  → DevDock 托管进程
  → 查看状态、端口和日志
  → 停止、重启或再次运行
```

第一阶段只把前端和 Python 做完整。Java 不进入扫描器、配置器、执行器和界面验收范围，但底层模型不能写死为这两种项目，以便以后增加 Java 执行器时无需重做进程管理。

## 2. 产品决策

### 2.1 统一使用 Command，不区分 service 和 task

用户看到和配置的核心对象统一称为“命令”。

不再要求用户先判断一个命令是长期服务还是一次性任务，原因如下：

- `npm run build`、`python migrate.py` 会自行退出。
- `npm run dev`、`python -m uvicorn ...` 会持续运行。
- 同一个脚本可能因为参数或项目实现不同而表现不同。
- DevDock 可以根据进程是否仍在运行，可靠地决定显示“停止”还是“再次运行”。
- `service/task` 推断错误会污染默认命令、重启按钮和界面分组。

运行时行为统一为：

- 命令启动成功且进程未退出：状态为“运行中”，支持停止、重启、日志和打开地址。
- 命令以退出码 `0` 结束：状态为“已完成”，支持再次运行和查看日志。
- 命令以非 `0` 退出：状态为“执行失败”，支持再次运行和查看日志。
- 用户主动停止：状态为“已停止”，支持再次运行和查看日志。

未来如果确实需要描述运行意图，可增加可选高级字段：

```json
{
  "runMode": "auto"
}
```

候选值可为 `auto`、`long-running`、`one-shot`，但 MVP 中默认和推荐值始终是 `auto`，界面不要求用户配置。

### 2.2 点击命令直接运行，不显示确认弹窗

当前 `window.confirm` 形式的系统弹窗应从命令运行路径移除。

信任建立在以下动作之一：

- 用户主动添加了项目目录。
- 命令来自该项目根目录的 `package.json` scripts。
- 用户接受了 Python 扫描候选。
- 用户在 DevDock 中手动创建并保存了命令。
- 用户直接编辑并保存了 `.lumina/project.json`。

执行时前端仍只传：

```json
{
  "projectPath": "...",
  "commandId": "..."
}
```

前端不能传完整 shell 命令。后端必须在每次启动时重新读取项目配置或 `package.json`，根据 `commandId` 解析受控的程序与参数。

“重新读取配置”不是重新扫描整个磁盘，也不是重新加载所有项目。它只读取当前点击命令所依赖的少量文件，因此应默认保留，且不需要做用户开关：

- package 命令：读取当前项目的 `package.json` 和锁文件信息。
- 配置命令：读取 `.lumina/project.json`。
- Python 解释器与脚本：在执行前校验目标仍存在。

这样可以防止界面显示旧命令、实际执行另一条命令的配置漂移，同时不会造成明显启动延迟。

### 2.3 配置变化使用非打断式反馈

`configRevision` 不再用于弹出运行确认框，只用于检测界面数据是否过期。

推荐交互：

- 扫描后配置未变化：正常显示。
- `package.json` 或 `.lumina/project.json` 已变化：项目标题区域显示“命令已更新”轻量状态。
- 用户点击运行时：后端以磁盘最新配置为准；成功后界面刷新该项目。
- 命令已被删除：显示轻量错误通知，并立即刷新项目命令列表。
- 命令预览变化：在配置抽屉显示最新预览，不弹系统模态框。

MVP 不提供“每次运行前确认”开关。若后续用户确有多租户、共享项目或高风险命令需求，再在设置中增加全局可选策略。

### 2.4 自动扫描与用户配置的边界

命令分为两种来源，但在主界面统一显示：

1. 可直接运行的命令
   - `package.json` scripts。
   - 已保存到 `.lumina/project.json` 的命令。
2. 待接受的候选命令
   - Python 常见入口。
   - `.cmd`、`.bat`、`.ps1` 启动脚本。
   - 从 Python 项目元数据推断出的命令。

候选命令不能在用户不知情的情况下自动执行。用户点击“添加”后，候选才写入 `.lumina/project.json` 并成为普通命令。

不要扫描并展示所有 `.py` 文件。MVP 仅检查明确位置和常见入口。

## 3. MVP 范围

### 3.1 前端项目

必须支持：

- 识别根目录 `package.json`。
- 读取所有合法的 `scripts`。
- 根据锁文件识别 `npm`、`pnpm`、`yarn`、`bun`。
- 命令预览，例如 `pnpm run dev`。
- 点击直接运行。
- 支持路径中包含空格、中文和括号。
- 执行器不可用时给出明确错误，例如“未找到 pnpm”。
- `package.json` 变化后重新扫描能够增加、更新和删除命令。
- 同一项目同一命令保持单实例；再次点击运行时聚焦现有运行记录，或明确提供“重启”，不悄悄启动第二份。

MVP 不做：

- 自动递归发现 monorepo 中所有 workspace 包。
- 自动执行依赖安装。
- 自动执行 `npx` 下载。
- 解析任意 shell 文本并让前端直接提交执行。

对于 monorepo，第一版允许用户直接添加具体子项目目录；工作区扫描作为后续独立能力设计。

### 3.2 Python 项目

必须识别：

- `pyproject.toml`
- `requirements.txt`
- `Pipfile`
- `poetry.lock`
- `uv.lock`
- `manage.py`
- `main.py`
- `app.py`
- `server.py`
- `run.py`
- 项目根目录和 `scripts/` 下的 `.cmd`、`.bat`、`.ps1`

必须支持的执行器：

```text
python          <interpreter> <script> <args...>
python-module   <interpreter> -m <module> <args...>
cmd             cmd.exe /D /S /C <script> <args...>
powershell      powershell.exe -NoProfile -File <script> <args...>
```

解释器解析顺序：

1. `.lumina/project.json` 中明确配置的解释器。
2. `.venv\Scripts\python.exe`。
3. `venv\Scripts\python.exe`。
4. 系统 `python`。

当前产品运行在 Windows，因此 MVP 优先保证 Windows 虚拟环境。跨平台解释器路径可以在模型中兼容，但不作为本轮验收阻塞项。

候选策略：

- `manage.py`：高置信度候选，默认参数为 `runserver`。
- `main.py`、`app.py`、`server.py`、`run.py`：中等置信度候选，用户接受前展示完整命令预览。
- `.cmd/.bat/.ps1`：中等置信度候选，只扫描根目录和 `scripts/`。
- `pyproject.toml` 中能安全映射为模块执行的入口：候选。
- `module:function` 类型的 console script 不得错误转换成 `python -m module`。

`uv` 和 Poetry 的设计预留如下，但建议排在前端/Python基本链路稳定之后：

```text
uv       uv run <program> <args...>
poetry   poetry run <program> <args...>
```

如果手头 Python 项目现阶段都能通过 `.venv`、Python、CMD 或 PowerShell 启动，则这两个执行器不应阻塞 MVP。

### 3.3 可视化配置

每个项目提供一个“命令设置”检查器式侧栏，不使用大型多层模态框。

侧栏结构：

```text
项目设置
  项目名称
  默认命令
  Python 解释器（仅 Python 项目显示）
  项目工作目录
  项目环境变量

已配置命令
  命令名称
  执行方式
  脚本或模块
  参数
  工作目录
  环境变量
  命令预览
  删除

发现的建议
  候选名称 + 来源 + 原因 + 预览
  [添加]

底部操作
  [取消] [保存]
```

交互原则：

- 不显示 service/task 选择器。
- 默认只显示常用字段；环境变量、工作目录等放在“高级设置”。
- 参数继续使用数组编辑，不提供一整段自由 shell 文本。
- 修改字段后实时生成只读命令预览。
- 校验错误贴近字段显示，不使用系统弹窗。
- 保存成功后关闭或保持侧栏由用户操作决定，并用轻量通知反馈。
- 自动候选添加后仍需用户点击“保存”才写盘。

## 4. 配置协议 v2

建议把当前配置升级为 `schemaVersion: 2`，原因是核心语义已经从 service/task 改为统一命令，继续复用 v1 字段会长期留下错误概念。

示例：

```json
{
  "schemaVersion": 2,
  "name": "ami-insight",
  "types": ["python"],
  "workingDirectory": ".",
  "environment": {
    "PYTHONUNBUFFERED": "1"
  },
  "runtimes": {
    "python": {
      "interpreter": ".venv\\Scripts\\python.exe"
    }
  },
  "commands": [
    {
      "id": "start-backend",
      "name": "启动后端",
      "executor": "cmd",
      "script": "start-backend.cmd",
      "args": [],
      "runPolicy": "singleton"
    },
    {
      "id": "uvicorn",
      "name": "Uvicorn",
      "executor": "python-module",
      "module": "uvicorn",
      "args": ["app:app", "--host", "127.0.0.1", "--port", "8000"],
      "runPolicy": "singleton"
    }
  ],
  "defaults": {
    "commandId": "start-backend"
  }
}
```

协议变化：

| v1                          | v2                         | 处理方式                           |
| --------------------------- | -------------------------- | ---------------------------------- |
| `kind: service/task`        | 删除，或只作为未知字段忽略 | 读取 v1 时接受，保存 v2 时不再写出 |
| `defaults.serviceCommandId` | `defaults.commandId`       | 自动迁移                           |
| `commandOverrides.*.kind`   | 删除                       | 读取时忽略，保存时清理             |
| `ProjectProcessMeta.kind`   | 删除                       | UI 根据进程状态决定操作            |

迁移规则：

- 后端读取 v1 和 v2。
- v1 在内存中转换为 v2 结构。
- 用户没有保存配置时，不主动改写文件。
- 用户下次通过可视化配置保存时，写为 v2。
- 保存仍采用临时文件、备份和失败回滚。
- 迁移不能修改 `package.json`。

注意：当前实现中前端已经把“默认服务”改成“默认命令”，但后端仍要求默认项必须是 service。这是 P0 阻塞问题，必须与弹窗移除一起修正，不能只改样式。

## 5. 执行与安全边界

### 5.1 允许的输入

前端只提交项目路径和命令 ID。后端负责：

- 读取最新配置。
- 查找命令。
- 校验命令 ID。
- 从枚举映射执行器。
- 将参数作为数组传给 `Command`。
- 合并项目和命令环境变量。
- 设置工作目录。
- 捕获 stdout 和 stderr。

### 5.2 路径约束

- 工作目录必须位于项目根目录内。
- Python、CMD、BAT、PowerShell 脚本必须位于项目根目录内。
- 相对 Python 解释器路径必须能解析为项目内的真实文件。
- 允许明确配置的绝对 Python 解释器，但执行前必须存在。
- 禁止 `..` 或符号链接绕过项目根目录约束。

### 5.3 进程窗口

所有托管命令在 Windows 上都应使用 `CREATE_NO_WINDOW`，并将 stdin 置空、stdout/stderr 管道化。

需要同时检查：

- package manager 命令。
- Python 和 Python module。
- CMD/BAT。
- PowerShell。
- 停止进程树使用的辅助命令。

项目脚本自身如果使用 `start`、`Start-Process` 或其他脱离父进程的方式，可能产生独立窗口或无法被完整托管。配置界面应显示非阻塞警告，文档明确建议启动脚本保持前台运行。

## 6. 进程模型

### 6.1 逻辑命令与运行尝试分离

当前进程记录按 PID/运行 ID 累积，重复失败或重启会在右侧留下多个同名卡片。新模型需要区分：

```text
CommandKey = normalizedProjectPath + commandId
RunAttempt = 本次启动生成的唯一 runId
```

主进程面板每个 `CommandKey` 只显示当前或最新一次运行：

- 正在运行时显示当前运行。
- 已结束时显示最新一次结果。
- 重启成功后，新运行替换旧运行的主卡片。
- 重启失败但尚未创建进程时，不生成虚假 PID 卡片；错误回到原卡片或轻量通知。
- 需要查看旧结果时进入“运行历史”，不在主面板重复堆叠。

MVP 可以先保留有限内存历史：每个命令 5 次、全局最多 40 次。应用退出后不保证保留。

### 6.2 状态机

```text
starting
  ├─ spawn 失败 → failed（无 PID，包含系统错误日志）
  └─ spawn 成功 → running
                    ├─ exit 0 → succeeded
                    ├─ exit 非 0 → failed
                    └─ 用户停止 → stopped
```

每次运行至少记录：

- `runId`
- `commandKey`
- `projectPath`
- `projectName`
- `commandId`
- `commandName`
- `executor`
- `commandPreview`
- `workingDirectory`
- `pid`（spawn 成功后才有）
- `status`
- `startedAt`
- `exitedAt`
- `exitCode`
- `ports`
- `urls`
- `warning`

`pid` 应改为可空，避免 spawn 失败时为了显示记录而伪造 PID。

### 6.3 单实例策略

MVP 所有命令默认 `singleton`：

- 同一 `CommandKey` 已运行时，再次点击不启动第二份。
- 主界面应定位并高亮现有进程卡片。
- 用户想重新启动时点击“重启”。
- 已结束命令点击运行时创建新 RunAttempt，并替换主卡片。

## 7. 日志模型

### 7.1 日志必须包含生命周期信息

当前只有子进程 stdout/stderr，子进程未输出或解码失败时日志为空。新增 `system` 流：

```text
[system] 准备执行：cmd.exe /D /S /C ...
[system] 工作目录：D:\...
[system] 进程已启动，PID 12345
[stderr] ...
[system] 进程退出，退出码 1
```

若 spawn 失败：

```text
[system/error] 启动失败：系统找不到指定的文件。未找到 pnpm。
```

安全要求：

- 显示环境变量键名，不显示值。
- 日志中不主动展开秘密配置。
- 命令预览应对包含空格的参数做可读转义，但实际执行仍使用参数数组。

### 7.2 输出读取

- stdout 和 stderr 独立读取并按时间进入同一环形缓冲区。
- 不使用遇到一次 UTF-8 错误便结束整个读取线程的方式。
- Windows 本地编码或非法字节使用 loss-tolerant 解码，至少不能导致后续日志永久丢失。
- 保留 ANSI 颜色解析，但先安全转义 HTML。
- 日志达到上限后丢弃最旧记录，并保留已解析出的端口和 URL。

### 7.3 空日志状态

日志窗口根据进程状态显示不同内容：

- `starting/running` 且暂无输出：“进程正在运行，暂时没有输出”。
- `succeeded` 且暂无输出：“命令已完成，没有产生输出”。
- `failed` 且暂无子进程输出：必须显示系统失败原因，不允许显示“等待进程输出”。
- `stopped` 且暂无输出：“进程已停止，没有产生输出”。

## 8. 端口与打开地址

端口不参与停止进程。停止必须以 DevDock 保存的 PID 为根，终止受托管的进程树。

端口的用途只有：

- 展示服务监听端口。
- 生成“打开”按钮的 URL。
- 帮助用户识别端口占用。

MVP 首先继续从 stdout/stderr 中解析完整 URL 和常见端口格式。P1 可增加 Windows 上按 PID/进程树查询监听端口，解决服务不打印 URL 时“打开”按钮不可用的问题。

绝不能根据端口号直接杀进程，因为端口可能已经被别的程序占用，也可能在父子进程切换中发生变化。

## 9. 界面设计

遵循桌面应用和 macOS 风格的渐进披露：主界面紧凑、状态清晰，复杂配置进入右侧检查器。

### 9.1 项目卡片

```text
ami-insight                                      [设置] [刷新]
D:\ly_project\ami-insight                 PYTHON · 2 个命令

[● start-backend                          ▶]
[● start-evolution-worker                 ▶]

发现 1 个建议                                    [查看]
```

规则：

- 不分“启动方式”和“任务”两个区块。
- 命令按钮高度、宽度和交互一致。
- 状态圆点表达空闲、启动中、运行中、成功、失败。
- 命令过长时截断，悬停显示完整名称与命令预览。
- 默认命令排第一；固定命令其次；其余按用户排序选择。
- 项目扫描中只替换该项目的命令区域，不阻塞整个页面。

### 9.2 进程检查器

主区分为：

- 运行中
- 最近完成

但“最近完成”按逻辑命令去重，只显示每条命令的最新结果。每张卡片根据状态显示操作：

| 状态      | 主操作            | 次操作           |
| --------- | ----------------- | ---------------- |
| starting  | 日志              | 停止             |
| running   | 打开（有 URL 时） | 日志、重启、停止 |
| succeeded | 再次运行          | 日志             |
| failed    | 再次运行          | 日志             |
| stopped   | 再次运行          | 日志             |

失败卡片应直接显示一行失败摘要，例如“退出码 1”或“未找到 pnpm”，不要求用户先打开空日志才能知道原因。

### 9.3 反馈层级

- 普通成功：轻量 toast 或按钮状态变化。
- 字段校验：字段下方行内提示。
- 项目扫描错误：项目卡片内提示，可重试。
- 命令运行错误：命令状态 + 进程卡片摘要 + 日志。
- 删除项目、删除配置命令等不可逆操作：应用内确认浮层。
- 运行普通已信任命令：不确认、不弹系统窗口。

## 10. 分阶段实施计划

### 阶段 0：冻结范围与建立回归基线

目标：在继续改模型前，明确哪些现有行为必须保留。

涉及文件：

- `src/services/project/project-service.ts`
- `src-tauri/src/commands/project_models.rs`
- `src-tauri/src/commands/project_process.rs`
- `docs/devdock-project-commands.md`

任务：

1. 列出当前 v1 配置样例和真实项目样例。
2. 准备前端 fixture：npm、pnpm、yarn、bun 各一个最小项目。
3. 准备 Python fixture：普通脚本、虚拟环境路径、Django、CMD、PowerShell、无输出失败脚本。
4. 记录现有 `.lumina/project.json`，确认迁移不能修改 `package.json`。
5. 把 Java 相关内容从本轮任务与验收表中移除。

验收：范围文档中只有 frontend/python；旧配置样例可用于迁移测试。

### 阶段 1：统一 Command 协议并迁移 v1

目标：彻底解除运行逻辑和 service/task 的耦合。

涉及文件：

- `src-tauri/src/commands/project_models.rs`
- `src-tauri/src/commands/project_config.rs`
- `src-tauri/src/commands/project_resolver.rs`
- `src-tauri/src/commands/project_executor.rs`
- `src/services/project/project-service.ts`
- `src/views/devdock/components/DevDockCommandConfigDrawer.vue`

任务：

1. 增加 v2 配置模型和 v1 兼容反序列化。
2. 删除 v2 `kind` 必填字段。
3. 将默认字段改为 `defaults.commandId`。
4. v1 `serviceCommandId` 自动映射到 v2 `commandId`。
5. 删除“默认命令必须是 service”的校验。
6. 删除 package script 的 kind 推断及 override kind。
7. 从前端类型和配置表单移除 kind。
8. 更新命令文档和 JSON 示例。
9. 增加迁移、默认命令、重复 ID、路径越界测试。

验收：旧 v1 配置可读取；保存后为 v2；默认命令可指向 build；`package.json` 不被写入。

### 阶段 2：移除运行确认并稳定信任边界

目标：点击已展示的命令立即运行，无 Windows 确认弹窗。

涉及文件：

- `src/views/devdock/DevDockView.vue`
- `src/i18n/messages/zh-CN.ts`
- `src/i18n/messages/en-US.ts`
- `src-tauri/src/commands/project_executor.rs`

任务：

1. 删除 `confirmProjectCommand()`。
2. 删除 `DEVDOC_CONFIRMED_COMMANDS_STORAGE_KEY` 及旧 localStorage 数据依赖。
3. 删除运行确认文案。
4. 保留 `projectPath + commandId` 的调用协议。
5. 后端每次启动重新读取配置并校验路径、执行器和脚本存在性。
6. 命令不存在或配置变化时返回结构化错误码，前端刷新当前项目。
7. 将配置变化反馈改为项目卡片内状态或轻量 toast。

验收：点击 package/Python/CMD/PowerShell 命令均无确认弹窗；删除或篡改命令后不会执行旧缓存命令。

### 阶段 3：修复进程模型和失败记录

目标：同一命令在右侧只有一个主记录，重启与再次运行语义清晰。

涉及文件：

- `src-tauri/src/commands/project_process.rs`
- `src/services/project/project-service.ts`
- `src/views/devdock/DevDockView.vue`
- `src/views/devdock/components/DevDockProcessPanel.vue`

任务：

1. 引入稳定 `commandKey`。
2. 将 PID 改为 spawn 成功后才存在。
3. 区分逻辑命令、当前 RunAttempt 和历史 RunAttempt。
4. 列表接口默认返回每个 commandKey 最新一次记录。
5. 重启时替换当前记录，不额外堆同名卡片。
6. spawn 失败写入失败尝试，但不伪造 PID。
7. 增加“清除最近完成”能力，运行中的记录不可被清除。
8. UI 操作按钮完全根据状态决定，不读取 kind。
9. 增加重复失败、重复运行、重启、停止的状态机测试。

验收：连续失败 3 次，右侧主面板仍只有一个该命令卡片；日志可切到对应最新失败；运行中不会启动第二实例。

### 阶段 4：补齐可靠日志

目标：任何失败都能从 UI 得到原因，不再出现“退出码 1 但日志为空且一直等待”。

涉及文件：

- `src-tauri/src/commands/project_process.rs`
- `src/services/project/project-service.ts`
- `src/views/devdock/components/DevDockLogModal.vue`
- `src/i18n/messages/zh-CN.ts`
- `src/i18n/messages/en-US.ts`

任务：

1. 日志流增加 `system`。
2. 启动前写命令预览和工作目录。
3. spawn 成功写 PID。
4. spawn 失败写操作系统错误。
5. 进程退出写退出码和时间。
6. 使用容错字节解码，避免单行编码错误终止读取。
7. 为 running/succeeded/failed/stopped 分别设计空日志文案。
8. 失败卡片直接显示最后一条 system/stderr 摘要。
9. 增加无输出成功、无输出失败、非 UTF-8 输出和大量日志淘汰测试。

验收：失败命令即使没有业务 stdout/stderr，日志中也至少包含执行命令、目录和失败原因或退出码。

### 阶段 5：完善前端项目扫描与执行

目标：npm/pnpm/yarn/bun 项目达到日常可用。

涉及文件：

- `src-tauri/src/commands/project.rs`
- `src-tauri/src/commands/project_resolver.rs`
- `src-tauri/src/commands/project_executor.rs`
- `src/views/devdock/DevDockView.vue`
- `src/views/devdock/components/DevDockProjectList.vue`

任务：

1. 明确锁文件优先级和 packageManager 字段优先级。
2. 扫描所有安全 script 名称。
3. 执行前检查 package manager 是否可用。
4. 禁止 npx 自动下载依赖。
5. package.json 变化后做增量项目刷新。
6. 处理空 scripts、非法 JSON、缺少包管理器的错误状态。
7. 覆盖空格、中文路径及特殊但合法 script 名称。

验收：四种包管理器 fixture 均能正确展示预览；缺少执行器时错误明确；package.json 更新后界面同步。

### 阶段 6：完善 Python 扫描与可视化配置

目标：常见 Windows Python 项目能够被识别、接受候选、配置并运行。

涉及文件：

- `src-tauri/src/commands/project_discovery.rs`
- `src-tauri/src/commands/project_config.rs`
- `src-tauri/src/commands/project_executor.rs`
- `src/views/devdock/components/DevDockCommandConfigDrawer.vue`

任务：

1. 用正式 TOML 解析替换逐行手写解析。
2. 增加 Python 项目标志文件识别。
3. 按既定顺序解析解释器。
4. 扫描常见入口与有限脚本目录。
5. 对 `module:function` 明确标为当前不可直接转换的候选，不误执行。
6. 候选去重并生成稳定 ID。
7. 候选添加时生成完整 v2 command 草稿。
8. 配置侧栏加入 Python 解释器、执行器相关字段和实时预览。
9. 校验脚本、模块、工作目录、参数和环境变量。
10. 补充 Django、普通脚本、CMD、PowerShell 和错误解释器测试。

验收：ami-insight 一类项目可通过 CMD/PowerShell 或 Python 入口稳定启动；`.venv` 自动识别；错误解释器有明确提示。

### 阶段 7：端口、视觉和最终回归

目标：完成桌面端交互收尾，形成可交付 MVP。

涉及文件：

- `src/views/devdock/components/DevDockProjectList.vue`
- `src/views/devdock/components/DevDockProcessPanel.vue`
- `src/views/devdock/components/DevDockLogModal.vue`
- `src/styles/workbench/index.scss`
- `src/i18n/messages/zh-CN.ts`
- `src/i18n/messages/en-US.ts`

任务：

1. 项目卡片统一命令列表，不展示 service/task 分组。
2. 收紧命令按钮尺寸和信息密度。
3. 运行、成功、失败、停止状态统一视觉语言。
4. 进程面板按最新逻辑命令去重。
5. 日志窗口修复空状态和失败摘要。
6. 保持应用内侧栏、popover 和 toast，不新增系统对话框。
7. 检查亮色/暗色主题、键盘焦点、禁用状态、文本截断。
8. 增加端口解析回归；按 PID 查询端口若延期，明确记录为 P1。
9. 按验收矩阵进行人工桌面验证。

验收：主流程没有阻塞式确认；命令列表比当前 service/task 分组更紧凑；失败原因可见；重复运行不会产生重复主卡片。

## 11. 测试与验收矩阵

| 场景                  | 预期结果                                |
| --------------------- | --------------------------------------- |
| npm 项目              | 扫描 scripts，点击直接运行，无弹窗      |
| pnpm/yarn/bun 项目    | 使用正确包管理器，预览与实际执行一致    |
| package.json 修改     | 刷新后命令增删同步，不修改 package.json |
| `.venv` Python 项目   | 自动选择项目解释器                      |
| Python 普通脚本       | 接受候选后写入配置并可运行              |
| `python -m uvicorn`   | 参数数组正确传递，路径含空格仍可运行    |
| Django                | `manage.py runserver` 候选正确          |
| CMD/BAT               | 无额外控制台窗口，日志可捕获            |
| PowerShell            | `-NoProfile -File` 执行，日志可捕获     |
| 执行器不存在          | 显示明确错误，不留下伪 PID              |
| 命令退出码 0          | 状态为已完成，可再次运行                |
| 命令退出码 1 且无输出 | 日志含 system 失败信息，不显示等待      |
| 连续失败多次          | 主面板仅一个最新卡片，历史有限保留      |
| 持续服务              | 支持日志、停止、重启和打开地址          |
| 停止进程              | 按 PID 终止进程树，不根据端口杀进程     |
| v1 配置               | 可读取；下次保存迁移到 v2               |
| 非法路径              | 后端拒绝项目外脚本和工作目录            |
| 中文/空格路径         | 命令参数不依赖字符串拼接，能够正常运行  |

## 12. 推荐实施顺序与里程碑

不要先继续增加更多扫描器。当前最需要的是把已有执行链路变得可信。

推荐顺序：

1. **里程碑 A：运行链路可信**  
   完成阶段 1–4：统一 Command、移除弹窗、进程去重、可靠日志。
2. **里程碑 B：前端项目完整**  
   完成阶段 5：四类包管理器和 package.json 变化处理。
3. **里程碑 C：Python 项目完整**  
   完成阶段 6：解释器、候选、配置和常见入口。
4. **里程碑 D：桌面体验交付**  
   完成阶段 7：状态、端口、视觉和完整回归。

只有里程碑 A 完成后才建议继续扩充 Python 扫描。否则扫描出来的命令越多，空日志、重复记录和不一致协议会放大得越明显。

## 13. 延期项

以下内容不进入本轮：

- Java/Maven/Gradle/Spring Boot。
- Docker Compose。
- Go/Rust。
- monorepo workspace 递归发现。
- 自动安装前端或 Python 依赖。
- shell 任意命令字符串执行器。
- 持久化跨应用重启的完整日志历史。
- 根据端口停止进程。
- 远程主机、容器或 WSL 进程管理。

## 14. 完成定义

当以下条件全部成立，前端/Python MVP 才算完成：

- 添加项目后可以识别前端或 Python 类型。
- 前端 scripts 可直接运行，Python 候选可接受并配置。
- 点击运行不再出现 Windows 确认弹窗。
- 前端不能提交任意完整命令，后端每次按命令 ID 解析最新配置。
- UI 和配置协议不再依赖 service/task。
- 同一命令在主进程面板只有一个当前或最新记录。
- 任何启动失败都能看到有用的系统日志或错误摘要。
- 持续进程可以停止和重启，一次性命令可以再次运行。
- 端口只用于展示和打开地址，不参与停止。
- 现有 v1 配置可兼容迁移，`package.json` 不会被 DevDock 修改。
- Java 明确不在本轮实现中。
