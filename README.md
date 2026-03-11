🦞📡 ClawMonitor (龙虾雷达)

"Monitor your Lobster. Secure your wallet. Look like a hacker."

赛博朋克风的 OpenClaw 本地代理防火墙与终端监控大屏。用最极客的方式，彻底掌控你的自主 AI 助手。

OpenClaw 极其强大，但让它在本地裸奔不仅面临隐私泄露风险，还可能一夜之间烧光你的 API Token。ClawMonitor 是一个由 Rust 编写的异步并发本地网关，附带一个高帧率的终端图形界面（TUI）。它能实时拦截异常请求、进行隐私脱敏，并把 Token 消耗可视化。

📸 运行截图 (Screenshot)

[ClawMonitor TUI](screenshot.png) 

终端就是男人的浪漫。实时柱状图，毫秒级日志刷新。

✨ 核心特性 (Features)

🖥️ 硬核终端 UI (Cyberpunk TUI): 基于 Ratatui 打造的高性能终端大屏，数据流可视化。

🛡️ 双核异步架构 (Async Architecture): 后台 Axum 极速代理服务器与前台 UI 渲染引擎完全分离，通过 MPSC 通道（Message Channels）实现无阻塞通信。

💰 Token 动态监控 (Token Tracker): 实时估算每次请求消耗的 Token 数量，动态生成近期流量柱状图。

🛑 隐私拦截与熔断 (Smart Firewall): 识别危险指令（如读取密码文件）、阻断超长无意义 prompt 暴走，物理掐断网线保护钱包。

🚀 快速开始 (Quick Start)

1. 编译与启动

确保你已安装 Rust。由于包含了 TUI 框架，推荐使用 --release 模式获得最丝滑的渲染帧率：

git clone [https://github.com/yourusername/clawmonitor.git](https://github.com/yourusername/clawmonitor.git)
cd clawmonitor
cargo run --release


快捷键操作：

按 t : 发送一次虚拟的网络测试请求（用于欣赏 UI 动画和测试防线）。

按 q : 优雅退出程序。

2. 将 OpenClaw 接入雷达

修改你的 OpenClaw (或其他任意大模型 Agent) 配置文件，将其 API Base URL 指向 ClawMonitor 的监听地址：

# 将原有的 OpenAI/第三方 API 代理替换为 ClawMonitor 本地地址
OPENAI_API_BASE=[http://127.0.0.1:8080/v1](http://127.0.0.1:8080/v1)


现在，你的 AI 助手的每一次心跳、每一笔 Token 支出，都将呈现在你的终端大屏上。

🏗️ 架构解析 (Under the Hood)

本项目是学习 Rust 并发编程的绝佳示例：

Tokio 异步任务: 代理服务器运行在一个独立的 Tokio 线程中。

MPSC Unbounded Channel: 网络层拦截到的数据，封装成 UiMessage 枚举，瞬间跨线程投递给主 UI 循环。

Crossterm & Ratatui: 接管终端接管原始模式（Raw Mode），进行 60fps 级别的屏幕重绘，毫秒级响应状态机 (AppState) 的变化。

🤝 参与贡献 (Contributing)

发现 bug？有更酷的 UI 排版点子？想接入真实的 OpenAI 转发逻辑？欢迎提交 Issue 或 Pull Request！

License: MIT