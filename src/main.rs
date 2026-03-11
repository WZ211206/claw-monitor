use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Local};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BarChart, List, ListItem, Paragraph},
    Terminal,
};
use serde_json::Value;
use std::{error::Error, io, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

// ==========================================
// 1. 数据模型与系统状态定义
// ==========================================

/// 定义一条拦截日志
#[derive(Clone)]
struct LogEntry {
    timestamp: DateTime<Local>,
    status: String,
    action: String,
    estimated_tokens: u64,
}

/// 发送给 UI 的消息类型
enum UiMessage {
    NewRequest(LogEntry),
}

/// 终端 UI 的状态管理
struct AppState {
    logs: Vec<LogEntry>,
    total_requests: u64,
    blocked_requests: u64,
    total_tokens: u64,
    token_history: Vec<(String, u64)>, // 用于柱状图
}

impl AppState {
    fn new() -> Self {
        Self {
            logs: Vec::new(),
            total_requests: 0,
            blocked_requests: 0,
            total_tokens: 0,
            token_history: vec![
                ("10s".to_string(), 0),
                ("8s".to_string(), 0),
                ("6s".to_string(), 0),
                ("4s".to_string(), 0),
                ("2s".to_string(), 0),
                ("Now".to_string(), 0),
            ],
        }
    }

    fn add_log(&mut self, log: LogEntry) {
        self.total_requests += 1;
        self.total_tokens += log.estimated_tokens;
        if log.status == "BLOCKED" {
            self.blocked_requests += 1;
        }

        // 维护日志列表长度（最多显示 20 条）
        self.logs.insert(0, log.clone());
        if self.logs.len() > 20 {
            self.logs.pop();
        }

        // 更新模拟的 Token 柱状图历史
        self.token_history.remove(0);
        self.token_history.push(("Now".to_string(), log.estimated_tokens));
        // 重命名 X 轴标签以制造流动效果
        for (i, label) in ["10s", "8s", "6s", "4s", "2s", "Now"].iter().enumerate() {
            self.token_history[i].0 = label.to_string();
        }
    }
}

// ==========================================
// 2. 异步网络代理服务器 (后台线程)
// ==========================================

async fn run_proxy_server(tx: UnboundedSender<UiMessage>) {
    // 使用 Arc 共享发送通道
    let shared_tx = Arc::new(tx);

    let app = Router::new()
        .route("/v1/chat/completions", post(handle_request))
        .with_state(shared_tx);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    // 启动 Web 服务
    axum::serve(listener, app).await.unwrap();
}

/// 处理拦截到的 OpenClaw 请求
async fn handle_request(
    State(tx): State<Arc<UnboundedSender<UiMessage>>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    // 简单估算 Token (字符长度 / 4)
    let payload_str = payload.to_string();
    let estimated_tokens = (payload_str.len() / 4) as u64;

    // 模拟基于关键字的隐私拦截逻辑
    let (status, action) = if payload_str.contains("secret") || payload_str.contains("password") {
        ("BLOCKED", "隐私阻断：包含敏感关键字")
    } else if estimated_tokens > 5000 {
        ("BLOCKED", "熔断：单次请求 Token 超限 (>5000)")
    } else {
        ("PASSED", "安全放行")
    };

    // 将事件通过通道发送给 UI 线程
    let log_entry = LogEntry {
        timestamp: Local::now(),
        status: status.to_string(),
        action: action.to_string(),
        estimated_tokens,
    };
    let _ = tx.send(UiMessage::NewRequest(log_entry));

    if status == "BLOCKED" {
        return (
            StatusCode::FORBIDDEN,
            "🦀 ClawMonitor: Request blocked for security reasons.",
        ).into_response();
    }

    // 模拟成功响应
    let mock_response = serde_json::json!({
        "id": "chatcmpl-clawmonitor-mock",
        "object": "chat.completion",
        "choices": [{"message": {"role": "assistant", "content": "Command received safely."}}]
    });

    (StatusCode::OK, Json(mock_response)).into_response()
}

// ==========================================
// 3. 终端 UI 主渲染循环 (前台线程)
// ==========================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化无界通道，用于 Proxy 和 UI 之间的通信
    let (tx, mut rx) = mpsc::unbounded_channel();

    // 在另一个 Tokio 任务中启动后台代理服务器
    tokio::spawn(async move {
        run_proxy_server(tx).await;
    });

    // --- 设置终端环境 ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 初始化 UI 状态
    let mut app_state = AppState::new();

    // 模拟插入几条初始数据让界面更好看
    let _ = app_state.add_log(LogEntry {
        timestamp: Local::now(),
        status: "SYSTEM".to_string(),
        action: "ClawMonitor 初始化完毕，监听端口 8080".to_string(),
        estimated_tokens: 0,
    });

    // --- 游戏/UI 主循环 ---
    loop {
        // 1. 渲染当前状态到终端屏幕
        terminal.draw(|f| draw_ui(f, &app_state))?;

        // 2. 检查是否有键盘输入事件 (非阻塞，超时 50 毫秒)
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // 如果按下 'q' 键，退出循环
                if key.code == KeyCode::Char('q') {
                    break;
                }
                // 如果按下 't' 键，我们手动触发一次虚拟请求用于测试 UI
                if key.code == KeyCode::Char('t') {
                    app_state.add_log(LogEntry {
                        timestamp: Local::now(),
                        status: "TEST".to_string(),
                        action: "收到模拟测试请求".to_string(),
                        estimated_tokens: fastrand::u64(100..2000),
                    });
                }
            }
        }

        // 3. 检查是否从后台 Proxy 收到了新的网络请求消息
        while let Ok(msg) = rx.try_recv() {
            match msg {
                UiMessage::NewRequest(log) => app_state.add_log(log),
            }
        }
    }

    // --- 恢复终端原状 (极其重要，否则退出后终端会乱码) ---
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// ==========================================
// 4. UI 绘制逻辑 (页面布局与组件)
// ==========================================

fn draw_ui(f: &mut ratatui::Frame, app: &AppState) {
    let size = f.size();

    // 将屏幕切分为三个部分: 顶部(状态)、中间(图表+日志)、底部(说明)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),      // 顶部面板高度 3
                Constraint::Min(10),        // 中间主体
                Constraint::Length(3),      // 底部说明高度 3
            ]
            .as_ref(),
        )
        .split(size);

    // --- 1. 顶部状态栏 ---
    let title_text = format!(
        " 🦞 ClawMonitor v0.1 | 总请求数: {} | 拦截次数: {} | 累计消耗 Token: {} ",
        app.total_requests, app.blocked_requests, app.total_tokens
    );
    let top_block = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title(" 全局状态 Dashboard "));
    f.render_widget(top_block, chunks[0]);

    // --- 2. 中间部分 (再次水平切分为图表和日志列表) ---
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[1]);

    // 2.a 左侧：Token 消耗柱状图
    let chart_data: Vec<(&str, u64)> = app.token_history.iter()
        .map(|(label, val)| (label.as_str(), *val))
        .collect();

    let barchart = BarChart::default()
        .block(Block::default().borders(Borders::ALL).title(" 近期 Token 流量 "))
        .data(&chart_data)
        .bar_width(5)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Yellow))
        .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));
    f.render_widget(barchart, middle_chunks[0]);

    // 2.b 右侧：实时请求日志列表
    let log_items: Vec<ListItem> = app.logs.iter().map(|log| {
        let time_str = log.timestamp.format("%H:%M:%S").to_string();
        
        // 根据状态决定颜色
        let status_color = match log.status.as_str() {
            "PASSED" => Color::Green,
            "BLOCKED" => Color::Red,
            "SYSTEM" => Color::Blue,
            _ => Color::Yellow, // TEST 等其他状态
        };

        let content = Line::from(vec![
            Span::styled(format!("[{}] ", time_str), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:>7} | ", log.status), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("消耗: ~{} Tokens | ", log.estimated_tokens), Style::default().fg(Color::Magenta)),
            Span::raw(log.action.clone()),
        ]);
        ListItem::new(content)
    }).collect();

    let logs_list = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title(" 📡 实时拦截日志 "));
    f.render_widget(logs_list, middle_chunks[1]);

    // --- 3. 底部按键提示栏 ---
    let bottom_text = "按 'q' 退出程序 | 按 't' 发送一次虚拟测试请求 | 将 OpenClaw API 指向 http://127.0.0.1:8080";
    let bottom_block = Paragraph::new(bottom_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(bottom_block, chunks[2]);
}