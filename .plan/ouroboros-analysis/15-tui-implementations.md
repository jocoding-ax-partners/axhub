# 15. TUI Implementations (Python + Rust)

## 두 구현 동시 운영

- **Python Textual** — `src/ouroboros/tui/` (28 파일)
- **Rust SuperLightTUI** — `crates/ouroboros-tui/` (별도 crate)

같은 SQLite EventStore 폴링. CLI flag `--backend python|slt` 로 선택.

## Python TUI (`src/ouroboros/tui/`)

### 진입

```bash
ouroboros tui monitor                  # 기본
ouroboros tui monitor --backend python
ouroboros monitor                       # shorthand
```

### 모듈

```
tui/
├─ app.py                   # OuroborosTUI Textual App, TUIState SSOT
├─ events.py                 # TUIState dataclass, message types
├─ screens/ (10)
│   ├─ dashboard.py
│   ├─ dashboard_v2.py
│   ├─ dashboard_v3.py        # 최신 버전
│   ├─ execution.py
│   ├─ logs.py
│   ├─ debug.py                # state inspector
│   ├─ lineage_detail.py
│   ├─ lineage_selector.py
│   ├─ session_selector.py
│   ├─ hud_dashboard.py
│   └─ confirm_rewind.py
├─ widgets/ (8)
│   ├─ ac_tree.py
│   ├─ ac_progress.py
│   ├─ agent_activity.py
│   ├─ cost_tracker.py
│   ├─ drift_meter.py
│   ├─ lineage_tree.py
│   ├─ parallel_graph.py
│   └─ phase_progress.py
└─ components/ (4)
    ├─ agents_panel.py
    ├─ event_log.py
    ├─ progress.py
    └─ token_tracker.py
```

### TUIState SSOT

`tui/events.py` 가 `TUIState` dataclass 정의 — 단일 진실 원천. `app.py` 가 보유.

이벤트 흐름:
```
EventStore (SQLite)
   ↓ poll 0.5s
app._subscribe_to_events()
   ↓
create_message_from_event()
   ↓
post_message()        # Textual 메시지 시스템
   ↓
Widget update
```

### Screens

| 키 | Screen | 표시 |
|---|---|---|
| 1 | Dashboard | Phase progress, AC tree, live status |
| 2 | Execution | Timeline, phase outputs, events |
| 3 | Logs | Filterable log viewer (level coloring) |
| 4 | Debug | State inspector, raw events, config |
| s | Sessions | Browse + switch sessions |
| e | Lineage | Evolutionary lineage 시각화 |

### 성능

- Refresh rate: 500 ms 폴링
- Event processing: < 100 ms / update

## Rust TUI (`crates/ouroboros-tui/`)

### Cargo.toml

```toml
[package]
name = "ouroboros-tui"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
description = "Native TUI monitor for Ouroboros workflows, built with SuperLightTUI"
license = "MIT"

[[bin]]
name = "ouroboros-tui"
path = "src/main.rs"

[dependencies]
superlighttui = "0.7.1"
rusqlite = { version = "0.33", features = ["bundled"] }    # bundled SQLite
serde_json = "1"
```

### 진입

```bash
ouroboros-tui [monitor]
ouroboros-tui --db-path /tmp/o.db
ouroboros-tui --mock                    # 데모 모드
```

또는 Python CLI 가 호출: `ouroboros tui monitor --backend slt`.

### 구조

```
src/
├─ main.rs                   # 진입, Rose Pine 테마, 키 핸들링
├─ db.rs                     # rusqlite 직접 폴링
├─ mock.rs                   # 데모 데이터 생성
├─ state.rs                  # AppState, SessionInfo, ExecutionStatus
└─ views/
    ├─ mod.rs
    ├─ dashboard.rs
    ├─ execution.rs
    ├─ lineage.rs
    ├─ logs.rs
    └─ session_selector.rs
```

### Rose Pine 테마 (하드코딩)

```rust
fn ouroboros_theme() -> Theme {
    Theme {
        primary:        Color::Rgb(196, 167, 231),   // iris
        secondary:      Color::Rgb(49, 116, 143),    // pine
        accent:         Color::Rgb(246, 193, 119),   // gold
        text:           Color::Rgb(224, 222, 244),   // text
        text_dim:       Color::Rgb(110, 106, 134),   // muted
        border:         Color::Rgb(38, 35, 58),      // overlay
        bg:             Color::Rgb(25, 23, 36),      // base
        success:        Color::Rgb(156, 207, 216),   // foam
        warning:        Color::Rgb(246, 193, 119),   // gold
        error:          Color::Rgb(235, 111, 146),   // love
        selected_bg:    Color::Rgb(38, 35, 58),
        selected_fg:    Color::Rgb(224, 222, 244),
        surface:        Color::Rgb(31, 29, 46),      // surface
        surface_hover:  Color::Rgb(38, 35, 58),
        surface_text:   Color::Rgb(144, 140, 170),   // subtle
    }
}
```

### DB 폴링 (`db.rs`)

기본 경로: `~/.ouroboros/ouroboros.db` 또는 `--db-path`.

```rust
let db_path = args.iter().position(|a| a == "--db-path")
    .and_then(|i| args.get(i + 1))
    .map(PathBuf::from)
    .unwrap_or_else(|| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".ouroboros/ouroboros.db")
    });
```

폴링 빈도 — 매 30 ticks (≈ 3 초):
```rust
poll_counter += 1;
if let Some(ref mut conn) = ouro_db {
    if poll_counter % 30 == 0 {
        let new_events = conn.read_new_events();
        // ...
    }
}
```

이벤트 필터링 — 현재 세션 + 그 execution + lineage + observability:
```rust
let new_events: Vec<_> = new_events.into_iter().filter(|ev| {
    ev.aggregate_id == state.session_id
        || (!state.execution_id.is_empty() && ev.aggregate_id.starts_with(&state.execution_id))
        || ev.event_type.starts_with("lineage.")
        || ev.event_type.starts_with("observability.")
}).collect();
```

### 4 Tabs

| Index | 탭 | 내용 |
|---|---|---|
| 0 | Dashboard | AC tree + 진척 |
| 1 | Execution | timeline |
| 2 | Lineage | 세대 비교 |
| 3 | Sessions | 세션 선택 |

### 키바인딩

| 키 | 동작 |
|---|---|
| `q` | quit |
| `Ctrl+P` | command palette |
| `p` | pause |
| `r` | resume |
| `1` `2` `3` `4` | 탭 직접 선택 |
| `e` | lineage shortcut |
| `s` | sessions shortcut |
| `l` | log panel toggle (execution 탭) |

### Command Palette (`Ctrl+P`)

7 항목:
1. Dashboard
2. Execution
3. Lineage
4. Sessions
5. Pause execution
6. Resume execution
7. Quit

### 헤더 정보

```rust
ui.text("◆ OUROBOROS").bold().fg(accent);
ui.text(status_label).fg(status_color).bold();   // ● RUN / ⏸ PAUSE / ✓ DONE / ✖ FAIL
ui.text(format!("[{done}/{total} AC]")).bold();
ui.text(&state.elapsed);
ui.text(format!("${:.2}", state.cost.total_cost_usd)).fg(success);
ui.text(format!("{}k tok", state.cost.total_tokens / 1000));
ui.text(format!("iter {}", state.iteration));
```

`Goal` 라인 별도 (가장 중요한 컨텍스트):
```rust
ui.text("Goal ").fg(dim);
ui.text_wrap(&state.seed_goal).fg(text).bold();
```

### Drift Sparkline (탭바)

탭바 우측 상단 항상 보임:
```rust
ui.text("drift ").fg(dim);
ui.sparkline(state.drift.history.make_contiguous(), 8);
ui.text(format!(" {:.2}", state.drift.combined)).fg(
    if state.drift.combined < 0.1 { drift_success }
    else if state.drift.combined < 0.2 { drift_warning }
    else { drift_error }
);
```

### Footer 동적 힌트

```rust
let extra_keys: &[(&str, &str)] = match state.screen {
    Screen::Dashboard => &[("↑↓", "Navigate tree"), ("Enter", "Expand/Collapse")],
    Screen::Execution => &[("l", "Log panel"), ("↑↓", "Scroll")],
    Screen::Lineage => &[("↑↓", "Select lineage")],
    Screen::SessionSelector => &[("Enter", "Load session"), ("←→", "Page"), ("Esc", "Back")],
};
```

### Mock 모드

`--mock` 또는 DB 비어있으면 자동 mock 폴백:
```rust
if event_count == 0 {
    state.add_log(LogLevel::Warning, "db", "DB empty — loading demo data");
    mock::init_mock_state(&mut state);
}
```

`mock::tick_mock()` 매 폴링 시 가짜 이벤트 시뮬레이션 (auto_simulate + 일시정지 안 됐을 때).

## 빌드 + 배포

`release.yml` 가 5 cross-arch 바이너리 빌드:
- linux-x64
- linux-arm64
- macos-x64
- macos-arm64
- windows-x64

→ `actions/upload-artifact@v4` → `attach-tui-binaries` job 이 release 자산 첨부.

## Python ↔ Rust 비교

| 항목 | Python (Textual) | Rust (SLT) |
|---|---|---|
| 의존 | textual >= 1.0 | superlighttui 0.7.1 |
| 폴링 | 500 ms | ~3 s (30 ticks) |
| 색상 | Textual 기본 | Rose Pine 하드코딩 |
| 모드 | live only | live + mock |
| 화면 | 6 (dashboard 3 버전 포함) | 4 |
| 위젯 | 8 + 4 컴포넌트 | views 5 |
| 디버그 | state inspector 화면 | log panel |
| 설치 | `pip install ouroboros-ai[tui]` | release 바이너리 또는 `cargo build` |
| 메모리 | 50–100 MB | 작음 (Rust) |

## 사용자 선택 가이드

- 빠른 디버깅, 풍부한 위젯 → Python
- 가벼움, 빠른 시작 → Rust
- 둘 다 같은 DB 폴링 → 데이터 일관성 보장

## 검증

`tests/unit/tui/` 7 파일:
- app, cancelled_display, events, lineage_viewer, screens, session_selector_replay, widgets

`tests/unit/test_dashboard.py` (323 LOC).

Rust 는 별도 cargo test (확인 못 함).
