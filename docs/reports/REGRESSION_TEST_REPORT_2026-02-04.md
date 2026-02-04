# 回归测试报告（2026-02-04）

## 范围
- 前端：单元测试（Vitest）+ 覆盖率 + 生产构建（Vite）
- 后端：Rust 单元/集成/E2E/性能相关测试（含 `#[ignore]` 的 release 性能用例）+ doc-tests

## 基线信息
- 时间：2026-02-04 11:06:12 CST
- 分支/提交：`main@de9c18c`（`test: fix decision refresh test DB schema and helper imports`）
- 工具链：
  - Node：v25.5.0
  - npm：11.8.0
  - cargo：1.85.1
  - rustc：1.85.1

## 执行命令与结果

### 前端
1) `npm run test:coverage`
- 结果：✅ 通过（5 files / 60 tests）
- 覆盖率汇总（v8）：
  - Statements：77.99%
  - Branches：58.97%
  - Functions：74.07%
  - Lines：81.26%

2) `npm run build`
- 结果：✅ 通过（`tsc && vite build`）

### 后端
1) `cargo test -- --quiet`
- 结果：✅ 通过
- 备注：包含大量集成/E2E/性能相关用例；其中一组性能测试运行时间较长（>60s 提示），但最终通过。

2) `cargo test --release --tests -- --ignored --quiet`
- 结果：✅ 通过（本次实际执行 4 个 ignored 用例，其余 test suites 无 ignored 用例）

3) `cargo test --release --doc`
- 结果：✅ 通过（15 passed / 11 ignored）

## 已知限制与注意事项
- 性能阈值类测试在 debug 环境下可能受机器负载波动影响；对 `#[ignore]` 的性能测试建议以 **release** 模式作为验收口径：`cargo test --release --tests -- --ignored`。
- 本次测试过程中 Rust 编译/运行输出存在一定数量的告警（unused import/variable、dead_code 等）；不影响功能正确性，但建议在后续“维护/稳定”收敛中逐步清理。

## 后续建议（TODO）
- 将回归命令沉淀为 `docs/guides/testing.md` 或脚本（例如 `scripts/test/regression.sh`），并在 CI 中固化（区分：快速 PR 校验 vs nightly 性能回归）。
- 对运行 >60s 的性能相关用例补充分层：小数据集 smoke + 大数据集 nightly，减少日常回归耗时与波动。
- 将 Rust warnings 作为渐进门禁（例如 `-D warnings` 先仅对新增代码生效），避免长期累积。
