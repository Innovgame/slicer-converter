# 本地格式参考源

后续新增或对齐 slicer format 行为时，优先参考本机相邻目录中的 UVTools 源码：

- UVTools 仓库相对路径：`../UVtools`
- 核心格式实现目录：`../UVtools/UVtools.Core/FileFormats/`
- 已用于本项目对齐的关键文件：
  - `../UVtools/UVtools.Core/FileFormats/SL1File.cs`
  - `../UVtools/UVtools.Core/FileFormats/VDTFile.cs`
  - `../UVtools/UVtools.Core/FileFormats/ChituboxZipFile.cs`
  - `../UVtools/UVtools.Core/FileFormats/AnycubicZipFile.cs`

涉及 entry naming、manifest/config 字段、版本矩阵、RLE/PNG layer codec、archive layout、metadata defaults 时，应先交叉阅读对应 UVTools 文件，再落本项目 spec/plan 和测试。

## 本地样本派生规则

后续如果缺少某类真实样本，先检查 `rust/data/` 中是否有可作为源数据的真实大样本，并优先通过本项目现有 CLI / codec / pipeline 派生生成可验证样本：

- 可由现有真实文件转换得到的格式，不要直接标记为“无数据”；例如缺 PRZ 时，可用 `rust/data/*.goo` 生成 `/tmp/*.prz` 后继续验证 PRZ 路径。
- 派生样本默认写入 `/tmp`，不要把大生成物提交进仓库；在 plan/spec 中记录源文件、生成命令、输出路径、layer count、route hit-rate 和 timing/benchmark 结果。
- 如果目标格式当前是 `can_export = false` 或 import-only（例如 Creality CXDLP v1/v2/v3），必须明确说明“无法由当前项目导出生成”，然后继续实现缺失的 export/manifest builder 或等待外部样本。Chitubox Zip 与 Anycubic Zip 当前已可导入可导出，不要再把它们当作 import-only。
- 所有派生样本仍需经过解析/反向转换/CLI smoke 或 Criterion semantic verification，不能只以“文件生成成功”作为验收。

## 注释与测试命名风格

新增或整理 TypeScript 代码时，注释应保持少量但有用，优先说明模块边界、导出契约、运行期状态和不容易从类型签名直接看出的约束：

- 生产代码中的导出类型、接口、入口函数和关键 runtime/session 状态，使用双语 JSDoc：

```ts
/**
 * @zh 当前工具与临时草图交互态；不进入 document world snapshot。
 * @en Current tool and transient sketch interaction state; it does not enter the document world snapshot.
 */
export interface WorkbenchToolInteractionSession {
    // ...
}
```

- 内部但有架构含义的导出契约可加 `@internal`，放在 `@zh` / `@en` 前：

```ts
/**
 * @internal
 * @zh 单个 converter document 的 runtime-local 状态。
 * @en Runtime-local state for one converter document.
 */
export interface ConverterDocumentSession {
    // ...
}
```

- 不给显而易见的局部变量、普通赋值或直观 JSX 文案加解释性注释；如果注释只是复述代码，应删除。
- 测试文件的 `describe` / `it` 名称优先采用英文在前、中文在后的形式：

```ts
describe("CommandManager / 命令管理器", () => {
    it("starts and completes a command / 启动并完成", async () => {
        // ...
    });
});
```

- 新增结构护栏、边界测试或行为回归测试时，也应使用上述双语测试命名，方便本地阅读和后续维护。
