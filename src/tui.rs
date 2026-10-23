use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::models::{TestResult, TestStatus};

#[derive(Debug, Clone)]
pub struct AppState {
    pub total_servers: usize,
    pub cached_count: usize,
    pub pending_count: usize,
    pub concurrency: usize,
    pub port: u16,
    pub ttl_display: String,
    pub mode: String,
    pub country_filter_str: String,
    pub feature_filter_str: String,

    pub tested_count: usize,
    pub online_count: usize,
    pub offline_count: usize,

    pub start_time: Instant,
    pub completed_time: Option<Instant>,
    pub is_paused: bool,
    pub is_completed: bool,
    pub status_message: String,

    pub results_map: HashMap<i64, TestResult>,
    pub recent_stream: VecDeque<TestResult>,
    pub top_servers: Vec<TestResult>,
}

impl AppState {
    pub fn new(
        total_servers: usize,
        cached_count: usize,
        pending_count: usize,
        concurrency: usize,
        port: u16,
        ttl_display: String,
        mode: String,
        country_filter_str: String,
        feature_filter_str: String,
    ) -> Self {
        Self {
            total_servers,
            cached_count,
            pending_count,
            concurrency,
            port,
            ttl_display,
            mode: mode.to_uppercase(),
            country_filter_str,
            feature_filter_str,

            tested_count: cached_count,
            online_count: 0,
            offline_count: 0,

            start_time: Instant::now(),
            completed_time: None,
            is_paused: false,
            is_completed: false,
            status_message: "Initializing bulk scanner...".to_string(),

            results_map: HashMap::new(),
            recent_stream: VecDeque::with_capacity(10),
            top_servers: Vec::new(),
        }
    }

    pub fn register_cached_results(&mut self, cached: HashMap<i64, TestResult>) {
        for (id, r) in cached {
            if r.status == TestStatus::Online {
                self.online_count += 1;
            } else {
                self.offline_count += 1;
            }
            self.results_map.insert(id, r);
        }
        self.update_top_servers();
    }

    pub fn record_result(&mut self, result: TestResult) {
        if let Some(prev) = self.results_map.get(&result.server_id) {
            if prev.status == TestStatus::Online {
                self.online_count = self.online_count.saturating_sub(1);
            } else {
                self.offline_count = self.offline_count.saturating_sub(1);
            }
        } else {
            self.tested_count += 1;
        }

        if result.status == TestStatus::Online {
            self.online_count += 1;
        } else {
            self.offline_count += 1;
        }

        self.results_map.insert(result.server_id, result.clone());

        if self.recent_stream.len() >= 10 {
            self.recent_stream.pop_back();
        }
        self.recent_stream.push_front(result);

        self.update_top_servers();

        if self.tested_count >= self.total_servers && !self.is_completed {
            self.is_completed = true;
            self.completed_time = Some(Instant::now());
            self.status_message = "All servers tested! Test completed successfully.".to_string();
        }
    }

    fn update_top_servers(&mut self) {
        let mut online: Vec<TestResult> = self
            .results_map
            .values()
            .filter(|r| r.status == TestStatus::Online && r.latency_ms.is_some())
            .cloned()
            .collect();

        online.sort_by(|a, b| {
            let la = a.latency_ms.unwrap_or(999999.0);
            let lb = b.latency_ms.unwrap_or(999999.0);
            la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
        });

        online.truncate(10);
        self.top_servers = online;
    }

    pub fn min_latency(&self) -> Option<f64> {
        self.results_map
            .values()
            .filter_map(|r| {
                if r.status == TestStatus::Online {
                    r.latency_ms
                } else {
                    None
                }
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn avg_latency(&self) -> Option<f64> {
        let lats: Vec<f64> = self
            .results_map
            .values()
            .filter_map(|r| {
                if r.status == TestStatus::Online {
                    r.latency_ms
                } else {
                    None
                }
            })
            .collect();
        if lats.is_empty() {
            None
        } else {
            let sum: f64 = lats.iter().sum();
            Some(sum / lats.len() as f64)
        }
    }

    pub fn max_latency(&self) -> Option<f64> {
        self.results_map
            .values()
            .filter_map(|r| {
                if r.status == TestStatus::Online {
                    r.latency_ms
                } else {
                    None
                }
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn elapsed_seconds(&self) -> f64 {
        let end = self.completed_time.unwrap_or_else(Instant::now);
        end.duration_since(self.start_time).as_secs_f64().max(0.1)
    }

    pub fn tests_per_sec(&self) -> f64 {
        let live_tested = self.tested_count.saturating_sub(self.cached_count);
        live_tested as f64 / self.elapsed_seconds()
    }

    pub fn eta_seconds(&self) -> Option<f64> {
        if self.is_completed || self.tested_count >= self.total_servers {
            return Some(0.0);
        }
        let rate = self.tests_per_sec();
        let remaining = self.total_servers.saturating_sub(self.tested_count);
        if rate > 0.0 {
            Some(remaining as f64 / rate)
        } else {
            None
        }
    }
}

pub fn format_duration(seconds: Option<f64>) -> String {
    match seconds {
        Some(s) => {
            let sec = s as u64;
            let m = sec / 60;
            format!("{:02}:{:02}", m, sec % 60)
        }
        None => "--:--".to_string(),
    }
}

pub fn get_latency_spans(latency_ms: Option<f64>) -> (String, Style) {
    match latency_ms {
        None => ("---".to_string(), Style::default().fg(Color::DarkGray)),
        Some(lat) => {
            let s = format!("{:5.1} ms", lat);
            if lat < 75.0 {
                (s, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else if lat < 150.0 {
                (s, Style::default().fg(Color::Cyan))
            } else if lat < 250.0 {
                (s, Style::default().fg(Color::Yellow))
            } else {
                (s, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            }
        }
    }
}

pub fn get_status_span(status: TestStatus, is_cached: bool) -> Span<'static> {
    if is_cached {
        Span::styled("[CACHED]", Style::default().fg(Color::Cyan))
    } else {
        match status {
            TestStatus::Online => Span::styled(
                "[ONLINE]",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            TestStatus::Timeout => Span::styled(
                "[TIMEOUT]",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            TestStatus::Refused => Span::styled(
                "[REFUSED]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            TestStatus::Unreachable => Span::styled(
                "[UNREACH]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            TestStatus::Pending => Span::styled("[PENDING]", Style::default().fg(Color::DarkGray)),
            TestStatus::Error => Span::styled("[ERROR]", Style::default().fg(Color::Red)),
        }
    }
}

pub fn build_progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return format!("[{}]   0.0%", " ".repeat(width));
    }
    let fraction = (current as f64 / total as f64).clamp(0.0, 1.0);
    let filled_len = ((width as f64) * fraction) as usize;
    let mut bar = "=".repeat(filled_len);
    if filled_len < width {
        bar.push('>');
        bar.push_str(&"-".repeat(width.saturating_sub(filled_len + 1)));
    }
    bar.truncate(width);
    let pct = fraction * 100.0;
    format!("[{}] {:5.1}%", bar, pct)
}

pub fn render_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.area();

    // Vertical layout: Header (3), Stats (5), Main (Fill), Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(size);

    // 1. Header Block
    render_header(frame, chunks[0], state);

    // 2. Dashboard Block
    render_dashboard(frame, chunks[1], state);

    // 3. Main Content
    render_main_content(frame, chunks[2], state);

    // 4. Footer Block
    render_footer(frame, chunks[3], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let mode_color = match state.mode.as_str() {
        "TLS" => Color::Yellow,
        _ => Color::Green,
    };

    let mut title_spans = vec![
        Span::styled(
            "NORDVPN BULK BENCHMARK ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[{}] ", state.mode),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
    ];

    if state.country_filter_str != "ALL" {
        title_spans.push(Span::styled(
            format!("[{}] ", state.country_filter_str),
            Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::styled(
        format!("[{}]", state.feature_filter_str),
        if state.feature_filter_str != "ALL" {
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        },
    ));

    let title_line = Line::from(title_spans);

    let meta_line = Line::from(vec![
        Span::styled("Servers: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ", state.total_servers),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Cached: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ", state.cached_count),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Pending: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ", state.pending_count),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Workers: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ", state.concurrency),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Port: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} ", state.port), Style::default().fg(Color::White)),
        Span::styled("TTL: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&state.ttl_display, Style::default().fg(Color::Green)),
    ]);

    let header_table = Table::new(
        vec![Row::new(vec![
            Cell::from(title_line),
            Cell::from(meta_line).style(Style::default().add_modifier(Modifier::empty())),
        ])],
        [Constraint::Percentage(45), Constraint::Percentage(55)],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header_table, area);
}

fn render_dashboard(frame: &mut Frame, area: Rect, state: &AppState) {
    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let frame_block = Block::default()
        .title("[ Session Statistics ]")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    frame.render_widget(frame_block, area);

    // Col 1: Progress
    let pct = if state.total_servers > 0 {
        state.tested_count as f64 / state.total_servers as f64 * 100.0
    } else {
        0.0
    };
    let col1_lines = vec![
        Line::from(vec![
            Span::styled("Tested:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{} ", state.tested_count, state.total_servers),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("({:.1}%)", pct), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Progress: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                build_progress_bar(state.tested_count, state.total_servers, 18),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(col1_lines), col_chunks[0]);

    // Col 2: Online / Offline
    let on_pct = if state.tested_count > 0 {
        state.online_count as f64 / state.tested_count as f64 * 100.0
    } else {
        0.0
    };
    let off_pct = if state.tested_count > 0 {
        state.offline_count as f64 / state.tested_count as f64 * 100.0
    } else {
        0.0
    };
    let col2_lines = vec![
        Line::from(vec![
            Span::styled("Online:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ", state.online_count),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("({:.1}%)", on_pct), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Offline: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} ", state.offline_count),
                if state.offline_count > 0 {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
            Span::styled(
                format!("({:.1}%)", off_pct),
                if state.offline_count > 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(col2_lines), col_chunks[1]);

    // Col 3: Latencies
    let min_lat = state
        .min_latency()
        .map(|v| format!("{:.1} ms", v))
        .unwrap_or_else(|| "---".to_string());
    let avg_lat = state
        .avg_latency()
        .map(|v| format!("{:.1} ms", v))
        .unwrap_or_else(|| "---".to_string());
    let max_lat = state
        .max_latency()
        .map(|v| format!("{:.1} ms", v))
        .unwrap_or_else(|| "---".to_string());

    let col3_lines = vec![
        Line::from(vec![
            Span::styled("Latency Min: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                min_lat,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Avg / Max:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(avg_lat, Style::default().fg(Color::Cyan)),
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
            Span::styled(max_lat, Style::default().fg(Color::Magenta)),
        ]),
    ];
    frame.render_widget(Paragraph::new(col3_lines), col_chunks[2]);

    // Col 4: Speed & Time
    let speed_str = format!("{:.1} /s", state.tests_per_sec());
    let elapsed_str = format_duration(Some(state.elapsed_seconds()));
    let eta_str = format_duration(state.eta_seconds());

    let col4_lines = vec![
        Line::from(vec![
            Span::styled("Speed:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                speed_str,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Elapsed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", elapsed_str), Style::default().fg(Color::White)),
            Span::styled("ETA: ", Style::default().fg(Color::DarkGray)),
            Span::styled(eta_str, Style::default().fg(Color::Yellow)),
        ]),
    ];
    frame.render_widget(Paragraph::new(col4_lines), col_chunks[3]);
}

fn render_main_content(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.width >= 100 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);

        render_stream_table(frame, columns[0], state);
        render_top_table(frame, columns[1], state);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        render_stream_table(frame, rows[0], state);
        render_top_table(frame, rows[1], state);
    }
}

fn render_stream_table(frame: &mut Frame, area: Rect, state: &AppState) {
    let header_row = Row::new(vec![
        Cell::from("Country").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Server").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("IP Address").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Latency").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Src").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let mut rows = Vec::new();
    if state.recent_stream.is_empty() {
        rows.push(Row::new(vec![
            Cell::from(Span::styled("Waiting for probes...", Style::default().fg(Color::DarkGray))),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
        ]));
    } else {
        let max_display = (area.height.saturating_sub(3) as usize).min(8).max(1);
        for r in state.recent_stream.iter().take(max_display) {
            let country_text = format!("[{}] {}", r.country_code, r.country_name);
            let (lat_str, lat_style) = get_latency_spans(r.latency_ms);
            let src_span = if r.is_cached {
                Span::styled("Cache", Style::default().fg(Color::Cyan))
            } else {
                Span::styled("Live", Style::default().fg(Color::Green))
            };

            rows.push(Row::new(vec![
                Cell::from(country_text),
                Cell::from(Span::styled(&r.name, Style::default().add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(&r.station, Style::default().fg(Color::DarkGray))),
                Cell::from(get_status_span(r.status, r.is_cached)),
                Cell::from(Span::styled(lat_str, lat_style)),
                Cell::from(src_span),
            ]));
        }
    }

    let stream_table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
            Constraint::Percentage(14),
            Constraint::Percentage(12),
            Constraint::Percentage(6),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title("[ Live Probe Stream (Latest Tests) ]")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    );

    frame.render_widget(stream_table, area);
}

fn render_top_table(frame: &mut Frame, area: Rect, state: &AppState) {
    let header_row = Row::new(vec![
        Cell::from("#").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Country").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Server").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Latency").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Load").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let mut rows = Vec::new();
    if state.top_servers.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("-"),
            Cell::from(Span::styled("No results yet", Style::default().fg(Color::DarkGray))),
            Cell::from("-"),
            Cell::from("-"),
            Cell::from("-"),
        ]));
    } else {
        let max_display = (area.height.saturating_sub(3) as usize).min(8).max(1);
        for (i, r) in state.top_servers.iter().take(max_display).enumerate() {
            let rank_style = if i == 0 {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if i < 3 {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let load_style = if r.load < 40 {
                Style::default().fg(Color::Green)
            } else if r.load < 75 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };

            let (lat_str, lat_style) = get_latency_spans(r.latency_ms);
            let country_text = format!("[{}] {}", r.country_code, r.country_name);

            rows.push(Row::new(vec![
                Cell::from(Span::styled(format!("#{}", i + 1), rank_style)),
                Cell::from(country_text),
                Cell::from(Span::styled(&r.name, Style::default().add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(lat_str, lat_style)),
                Cell::from(Span::styled(format!("{}%", r.load), load_style)),
            ]));
        }
    }

    let top_table = Table::new(
        rows,
        [
            Constraint::Percentage(8),
            Constraint::Percentage(26),
            Constraint::Percentage(36),
            Constraint::Percentage(18),
            Constraint::Percentage(12),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title("[ Top 10 Fastest Servers ]")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(top_table, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let (tag, tag_style) = if state.is_paused {
        ("[PAUSED] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else if state.is_completed {
        ("[COMPLETE] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        ("[TESTING] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    };

    let status_line = Line::from(vec![
        Span::styled(tag, tag_style),
        Span::styled(&state.status_message, Style::default().fg(Color::White)),
    ]);

    let controls_line = Line::from(vec![
        Span::styled("[Q]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(" Quit & Save  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[P]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" Pause/Resume  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[R]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Retest All  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[E]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" Export CSV/JSON", Style::default().fg(Color::DarkGray)),
    ]);

    let footer_table = Table::new(
        vec![Row::new(vec![
            Cell::from(status_line),
            Cell::from(controls_line),
        ])],
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer_table, area);
}
