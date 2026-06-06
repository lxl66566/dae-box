# dae-box

一个 [sing-box](https://github.com/SagerNet/sing-box) 的 wrapper，可以**使用 dae 配置启动 sing-box 代理**，等价于使用 dae 作为 sing-box 的前端。基于 [sing-dae](https://github.com/lxl66566/sing-dae) 实现配置转换。

- dae-box 在 Windows 上会自动设置系统代理。

## 安装

需要系统中已安装 `sing-box` 可执行文件。以下方式**任选其一**：

- 手动从 [release](https://github.com/lxl66566/dae-box/releases) 下载并安装
- 使用 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 安装：
  ```bash
  cargo binstall dae-box --git https://github.com/lxl66566/dae-box
  ```

## 用法

```bash
dae-box run -c config.dae
```

dae-box 会将 `config.dae` 转换为 sing-box JSON 配置，写入临时文件后启动 sing-box。也支持直接传入 sing-box 原生 JSON 配置：

```bash
dae-box run -c config.json
```

其他子命令（`check`、`format`、`tools`、`version`）均为透传，等同于直接调用 `sing-box`。

### 子命令

| 命令      | 说明                    |
| --------- | ----------------------- |
| `run`     | 转换配置并启动 sing-box |
| `check`   | 检查配置合法性          |
| `format`  | 格式化配置（透传）      |
| `tools`   | 工具（透传）            |
| `version` | 显示版本（透传）        |

## Windows 系统代理

当配置的 `inbounds` 中包含 `mixed` 或 `http` 类型入站时，dae-box 在 Windows 上会自动：

1. 启动前设置系统代理
2. 退出时（包括 Ctrl+C、异常退出）自动恢复原始代理设置

## Tested on

sing-box version 1.13.13, windows/amd64
