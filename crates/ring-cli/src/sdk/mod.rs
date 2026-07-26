//! SDK 模式：stdin/stdout NDJSON 双向通信，供 IDE / app 等宿主进程嵌入。
//!
//! 设计参照 VSCode agentHost 的 CLI-agent 接入模式（spawn 子进程 + stdio 流式
//! 双向协议）：宿主把 `ring --sdk` 当子进程拉起，按行写请求、按行读事件。
//!
//! # 上行（宿主 → ring，每行一个 JSON）
//!
//! ```jsonl
//! {"id":"<uuid>","type":"message","payload":"最新用户消息","model":"provider/model","history":[{"role":"user","content":"..."},{"role":"assistant","content":"..."}]}
//! {"id":"<uuid>","type":"models"}
//! {"id":"<uuid>","type":"abort"}
//! {"id":"<uuid>","type":"ping"}
//! {"id":"<uuid>","type":"exit"}
//! ```
//!
//! `message` 字段说明：
//! - `model`   可选，按请求选模（缺省用启动配置）。
//! - `history` 可选，完整会话历史（role/content 数组）；提供时以其为准（无状态），
//!   否则回退到 `payload` 作为单条用户消息。宿主侧（IDE）持有会话历史真相。
//!
//! # 下行（ring → 宿主，每行一个 JSON，实时输出）
//!
//! ```jsonl
//! {"id":"<uuid>","type":"ready","payload":"ring SDK ready — model: ..."}
//! {"id":"<uuid>","type":"models","payload":"[{\"id\":\"...\",\"role\":\"...\"}]"}
//! {"id":"<uuid>","type":"reasoning","payload":"增量推理文本"}
//! {"id":"<uuid>","type":"text","payload":"增量文本"}
//! {"id":"<uuid>","type":"tool_start","payload":"{\"call_id\":\"...\",\"tool\":\"bash\",\"input\":{...}}"}
//! {"id":"<uuid>","type":"tool_end","payload":"{\"call_id\":\"...\",\"tool\":\"bash\",\"ok\":true,\"duration_ms\":12}"}
//! {"id":"<uuid>","type":"done","payload":"end_turn"}
//! {"id":"<uuid>","type":"error","payload":"错误信息"}
//! {"id":"<uuid>","type":"aborted","payload":"cancelled"}
//! {"id":"<uuid>","type":"pong"}
//! ```
//!
//! # 并发模型
//!
//! - stdin 由独立 OS 线程读取，经 channel 送入主循环 —— 生成进行中也能响应
//!   `abort` / `models` / 新的 `message`（新 message 会先中断上一轮，VSCode 语义）。
//! - stdout 由独立 writer task 串行化输出 —— 多生产者（主循环 + 生成任务）
//!   不会交错写坏行。
//! - 每轮生成持有一个 `CancellationToken`，`abort` 触发后同时停掉执行器与事件转发。

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use ring_core::events::RingEvent;
use ring_core::tools::{ContentBlock, Message, MessageRole};
use ring_engine::AgentContext;
use ring_providers::provider::Provider;

use crate::args::Args;
use crate::bootstrap::{self, BootstrappedRuntime};

// ── 协议结构 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkInput {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub payload: String,
    /// 按请求选模（`provider/model` 或裸模型名）。缺省用启动配置的模型。
    #[serde(default)]
    pub model: Option<String>,
    /// 完整会话历史。提供时无状态重建上下文；否则回退 `payload` 单条用户消息。
    #[serde(default)]
    pub history: Option<Vec<HistoryMessage>>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct SdkOutput {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: String,
}

impl SdkOutput {
    fn new(id: Uuid, msg_type: &str, payload: impl Into<String>) -> Self {
        Self { id, msg_type: msg_type.into(), payload: payload.into() }
    }
}

/// 一轮进行中的生成，供 `abort` 与新一轮 `message` 抢占时取消。
struct GenHandle {
    cancel: CancellationToken,
    join: tokio::task::JoinHandle<()>,
}

// ── 入口 ─────────────────────────────────────────────────────────────────────

/// Run SDK mode: read NDJSON from stdin, execute agent, stream events to stdout.
pub async fn run(args: &Args) -> Result<()> {
    let runtime = bootstrap::bootstrap(args, None).await?;

    // stdout 串行化 writer：所有下行消息走 channel，避免多生产者交错。
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<SdkOutput>();
    let writer = tokio::spawn(async move {
        while let Some(out) = out_rx.recv().await {
            let line = serde_json::to_string(&out).unwrap_or_default();
            let _ = writeln!(io::stdout(), "{line}");
            let _ = io::stdout().flush();
        }
    });

    // stdin 读取线程：阻塞读行 → channel，主循环异步消费（生成中亦可收 abort）。
    let (in_tx, mut in_rx) = mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for raw in stdin.lock().lines() {
            match raw {
                Ok(l) if l.trim().is_empty() => continue,
                Ok(l) => {
                    if in_tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let ready_model = if runtime.model.is_empty() { "(none)".to_string() } else { runtime.model.clone() };
    let _ = out_tx.send(SdkOutput::new(
        Uuid::nil(),
        "ready",
        format!("ring SDK ready — model: {ready_model}"),
    ));

    let mut current_gen: Option<GenHandle> = None;

    while let Some(raw) = in_rx.recv().await {
        let input: SdkInput = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                let _ = out_tx.send(SdkOutput::new(Uuid::nil(), "error", format!("parse error: {e}")));
                continue;
            }
        };

        match input.msg_type.as_str() {
            "ping" => {
                let _ = out_tx.send(SdkOutput::new(input.id, "pong", String::new()));
            }
            "exit" | "stop" => break,
            "models" => {
                let models: Vec<serde_json::Value> = runtime
                    .catalog
                    .iter()
                    .map(|m| serde_json::json!({ "id": m.id, "role": m.role.as_str() }))
                    .collect();
                let payload = serde_json::to_string(&models).unwrap_or_else(|_| "[]".into());
                let _ = out_tx.send(SdkOutput::new(input.id, "models", payload));
            }
            "abort" => {
                if let Some(gen) = current_gen.as_ref() {
                    if !gen.join.is_finished() {
                        gen.cancel.cancel();
                        let _ = out_tx.send(SdkOutput::new(input.id, "aborted", "cancelled"));
                    }
                }
            }
            "message" => {
                // 新一轮抢占上一轮（VSCode 语义：新 turn 打断旧 turn）。
                if let Some(gen) = current_gen.take() {
                    if !gen.join.is_finished() {
                        gen.cancel.cancel();
                    }
                }
                current_gen = Some(spawn_generation(input, &runtime, out_tx.clone()));
            }
            other => {
                let _ = out_tx.send(SdkOutput::new(input.id, "error", format!("unknown type: {other}")));
            }
        }
    }

    // 收尾：停掉在途生成，关闭 writer。
    if let Some(gen) = current_gen.take() {
        gen.cancel.cancel();
        let _ = gen.join.await;
    }
    drop(out_tx);
    let _ = writer.await;

    Ok(())
}

// ── 生成任务 ─────────────────────────────────────────────────────────────────

/// 按请求解析 provider 与 model。`requested` 缺省时回退启动配置。
fn resolve_request(
    requested: Option<&str>,
    runtime: &BootstrappedRuntime,
) -> (Option<Arc<dyn Provider>>, String) {
    match requested {
        Some(m) if !m.trim().is_empty() => {
            let (prov_hint, model_name) = ring_providers::split_model_ref(m);
            let provider = prov_hint
                .as_deref()
                .and_then(|pid| runtime.provider_registry.get(pid))
                .or_else(|| runtime.provider.clone());
            let model = if model_name.is_empty() {
                provider
                    .as_ref()
                    .map(|p| p.default_model().to_string())
                    .unwrap_or_default()
            } else {
                model_name
            };
            (provider, model)
        }
        _ => (runtime.provider.clone(), runtime.model.clone()),
    }
}

/// 由会话历史（或单条 payload）构建无状态上下文。
fn build_context(input: &SdkInput, system_prompt: &str, model: &str) -> AgentContext {
    let mut ctx = AgentContext::new(model.to_string());
    ctx.system = Some(system_prompt.to_string());
    match &input.history {
        Some(history) if !history.is_empty() => {
            for m in history {
                let role = match m.role.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => continue,
                };
                ctx.add_message(Message::new(role, vec![ContentBlock::Text { text: m.content.clone() }]));
            }
        }
        _ => {
            ctx.add_message(Message::user_text(input.payload.clone()));
        }
    }
    ctx
}

/// 启动一轮生成：解析选模 → 克隆运行时部件 → spawn 任务并发跑执行器与事件转发。
///
/// `runtime` 归主循环所有，故所需字段在 spawn 前克隆/拷贝，任务完全自持。
fn spawn_generation(
    input: SdkInput,
    runtime: &BootstrappedRuntime,
    out_tx: mpsc::UnboundedSender<SdkOutput>,
) -> GenHandle {
    let id = input.id;
    let (provider, model) = resolve_request(input.model.as_deref(), runtime);

    // 订阅必须先于执行器启动，避免丢事件。
    let sub = runtime.bus.subscribe();
    let session_id = runtime.session.meta.id;

    // 克隆运行时部件，使生成任务自持（不借用主循环的 runtime）。
    let tools = runtime.tools.clone();
    let permissions = runtime.permissions.clone();
    let bus = runtime.bus.clone();
    let catalog = runtime.catalog.clone();
    let cwd = runtime.cwd.clone();
    let system_prompt = runtime.system_prompt.clone();

    let cancel = CancellationToken::new();
    let gen_cancel = cancel.clone();

    let join = tokio::spawn(async move {
        let Some(provider) = provider else {
            let _ = out_tx.send(SdkOutput::new(id, "error", "no provider configured"));
            return;
        };

        let ctx = build_context(&input, &system_prompt, &model);

        let executor = ring_engine::agent::orchestrator::build_executor(
            ring_engine::agent::orchestrator::ExecutorParams {
                tools,
                permissions,
                bus,
                catalog,
                current_model: model.clone(),
                session_id,
                cwd,
                max_tokens: ring_providers::provider::DEFAULT_MAX_OUTPUT_TOKENS as u64,
                provider,
                perm_tx: None,
            },
        );

        run_turn(executor, ctx, sub, session_id, id, cancel, out_tx).await;
    });

    GenHandle { cancel: gen_cancel, join }
}

/// 并发驱动执行器与事件转发，直到终结事件 / 取消 / 执行器退出。
///
/// 执行器 future 与 `bus` 转发在同一任务内 `select`；取消令牌同时停两者。
/// 执行器未发终结事件即退出时补发 `done` 兜底，避免宿主永久挂起。
async fn run_turn(
    executor: ring_engine::AgentExecutor,
    mut ctx: AgentContext,
    mut sub: broadcast::Receiver<RingEvent>,
    session_id: Uuid,
    id: Uuid,
    cancel: CancellationToken,
    out_tx: mpsc::UnboundedSender<SdkOutput>,
) {
    let exec_fut = executor.run(&mut ctx, cancel.clone());
    tokio::pin!(exec_fut);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = out_tx.send(SdkOutput::new(id, "aborted", "cancelled"));
                break;
            }
            _ = &mut exec_fut => {
                // 执行器未发终结事件即退出（如达到 max_turns）→ 补发 done 兜底。
                // 已发过终结事件的分支都会先 break，不会到达这里。
                let _ = out_tx.send(SdkOutput::new(id, "done", "end_turn"));
                break;
            }
            ev = sub.recv() => {
                match ev {
                    Ok(ev) => {
                        if ev.session_id() != session_id {
                            continue;
                        }
                        match &ev {
                            RingEvent::AgentReasoning { delta, .. } => {
                                let _ = out_tx.send(SdkOutput::new(id, "reasoning", delta.clone()));
                            }
                            RingEvent::AgentText { delta, .. } => {
                                let _ = out_tx.send(SdkOutput::new(id, "text", delta.clone()));
                            }
                            RingEvent::ToolStart { call_id, tool_name, input, .. } => {
                                let payload = serde_json::json!({
                                    "call_id": call_id,
                                    "tool": tool_name,
                                    "input": input,
                                })
                                .to_string();
                                let _ = out_tx.send(SdkOutput::new(id, "tool_start", payload));
                            }
                            RingEvent::ToolEnd { call_id, tool_name, ok, duration_ms, .. } => {
                                let payload = serde_json::json!({
                                    "call_id": call_id,
                                    "tool": tool_name,
                                    "ok": ok,
                                    "duration_ms": duration_ms,
                                })
                                .to_string();
                                let _ = out_tx.send(SdkOutput::new(id, "tool_end", payload));
                            }
                            RingEvent::AgentDone { stop_reason, .. } => {
                                let _ = out_tx.send(SdkOutput::new(id, "done", stop_reason.clone()));
                                break;
                            }
                            RingEvent::AgentError { error, .. } => {
                                let _ = out_tx.send(SdkOutput::new(id, "error", error.clone()));
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}
