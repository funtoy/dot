# dot

一个**编译期**读取配置的过程宏（proc-macro）。`dot!("KEY")` 在编译时从 `.env` 文件
或系统环境变量中取出值，并把它作为 `&'static str` 字面量**内联**进二进制——运行时没有任何
文件读取或解析开销，缺值则**编译报错**（而非运行时 panic）。

## 用法

```toml
# Cargo.toml
[dependencies]
dot = { path = "./dot" }
```

```rust
// 取值；编译期找不到则编译失败，用默认错误信息
let api_key: &str = dot::dot!("API_KEY");

// 第二个参数自定义“找不到”时的编译错误信息
let api_key: &str = dot::dot!("API_KEY", "请在 .env 里配置 API_KEY");
```

宏接受 1～2 个**字符串字面量**参数。传入非字面量（如 `dot!(FOO)`）会得到指向出错位置的
编译错误，而不是宏 panic。

## 取值来源与优先级

按以下顺序解析，先命中者胜：

1. **`.env` 文件**：从 `CARGO_MANIFEST_DIR` 开始逐级向上查找配置文件。
2. **系统环境变量**：`std::env::var(KEY)`（编译时所在进程的环境）。

两者都没有，则产生编译错误。文件是**可选**的——没有任何 `.env.*` 文件时，仅靠系统环境变量
也能工作。

### dev / prod 文件选择

根据编译模式选择文件名：

| 编译模式            | 文件名      |
| ------------------- | ----------- |
| debug（默认）       | `.env.dev`  |
| release             | `.env.prod` |

> ⚠️ 该选择基于 `dot` 这个 proc-macro crate 自身编译时的 `debug_assertions`。通常 `cargo build`
> 与 `cargo build --release` 会让它跟随切换，但 `debug-assertions` 是可被独立配置的
> （`[profile.release] debug-assertions = true`、`build-override`、自定义 `--profile`），
> 此时文件可能选错。需要稳健切换时，建议改用 cargo feature 或显式环境变量。

### `.env` 文件格式

```dotenv
# 以 # 开头的整行是注释
API_KEY=abc123
DATABASE_URL="postgres://localhost/db"   # 值两侧的成对引号会被去掉
PORT=8080
```

- `KEY=VALUE`，按第一个 `=` 分割；空行与 `#` 开头的行忽略。
- 值会 `trim`，并去掉首尾的 `"` 或 `'`。
- **不支持**行内注释（`KEY=val # 注释` 中的 `# 注释` 会算进值里）、转义、多行值。

## 注意事项

- **值会被写进二进制**：`dot!` 把配置值固化成字符串字面量编译进产物。不要用它嵌入生产密钥
  （二进制可被 `strings` 提取），且改配置需要**重新编译**。
- **文件变更会触发重编译**：取值成功且存在 `.env.*` 文件时，宏会发出
  `include_bytes!(<env 文件>)`，使 rustc 追踪该文件，文件内容变化即重新编译。
- **系统环境变量变更不触发重编译**：`std::env::var` 在 proc-macro 中不会被 rustc 登记为
  重编译依赖。改了环境变量但源码/文件未变时可能不会重新编译，必要时执行 `cargo clean`。

## 测试

仓库用 [`trybuild`](https://docs.rs/trybuild) 固化编译期行为（`tests/ui/`）：

```bash
cargo test                      # 运行通过/失败用例
TRYBUILD=overwrite cargo test   # syn 版本或错误信息变动后，重新生成 .stderr 快照
```
