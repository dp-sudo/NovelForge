# Task Statement
对当前 NovelForge 项目执行全链路深度静态与动态扫描分析，覆盖代码质量、数据流、架构设计、运行时行为、安全与性能风险，并形成分级问题清单和可执行重构路线。

# Desired Outcome
1. 分级问题清单（严重度/位置/表现/影响/建议）
2. 风险影响评估（业务影响 + 技术影响）
3. P0/P1/P2 可落地优化与技术债治理路线

# Known Facts / Evidence
- 当前仓库为 Tauri2 + React/Vite + Rust + SQLite 混合栈。
- 现有关键契约测试包括 tauri-contract-smoke 等。
- 工作树当前存在未提交改动，审计需避免回滚与误改。

# Constraints
- 只做审计分析与规划，不做越界重构。
- 结论必须绑定可验证证据（命令输出或代码定位）。

# Unknowns / Open Questions
- 当前主要风险是否集中在命令契约层、AI 流水线层还是 UI 状态流。
- 动态测试覆盖对边界条件是否充分。

# Likely Touchpoints
- src-tauri/src/lib.rs
- src-tauri/src/commands/*
- src-tauri/src/services/*
- src/api/*
- src/pages/*
- tests/integration/*