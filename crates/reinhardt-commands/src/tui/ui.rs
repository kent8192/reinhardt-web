use super::app::{AppState, Pane};
use ratatui::{
	Frame,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, Borders, Paragraph, Wrap},
};

/// Draw TUI screen
pub fn draw(frame: &mut Frame, state: &AppState) {
	let size = frame.area();

	// Main layout: status bar, log panes, control bar
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3), // Status bar
			Constraint::Min(0),    // Log panes (variable)
			Constraint::Length(3), // Control bar (borders + 1 line content)
		])
		.split(size);

	// Draw status bar
	draw_status_bar(frame, chunks[0], state);

	// Draw log panes (backend/frontend)
	draw_log_panes(frame, chunks[1], state);

	// Draw control bar
	draw_control_bar(frame, chunks[2], state);
}

/// Draw status bar
fn draw_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
	use super::metrics::ProcessStatus;

	// Backend status
	let (backend_status_text, backend_status_color) = match state.metrics.backend_status {
		ProcessStatus::Running => ("● Running", Color::Green),
		ProcessStatus::Crashed => ("● Crashed", Color::Red),
		ProcessStatus::NotStarted => ("○ Not Started", Color::Gray),
	};

	// Frontend status
	let (frontend_status_text, frontend_status_color) = match state.metrics.frontend_status {
		ProcessStatus::Running => ("● Running", Color::Green),
		ProcessStatus::Crashed => ("● Crashed", Color::Red),
		ProcessStatus::NotStarted => ("○ Not Started", Color::Gray),
	};

	let status_lines = vec![
		Line::from(vec![
			Span::styled("Backend: ", Style::default().fg(Color::Cyan)),
			Span::styled(
				backend_status_text,
				Style::default().fg(backend_status_color),
			),
			Span::raw(format!(
				"  Mem: {:.1}MB  CPU: {:.1}%",
				state.metrics.backend_memory_mb, state.metrics.backend_cpu_percent
			)),
		]),
		Line::from(vec![
			Span::styled("Frontend: ", Style::default().fg(Color::Cyan)),
			Span::styled(
				frontend_status_text,
				Style::default().fg(frontend_status_color),
			),
			Span::raw(format!(
				"  Mem: {:.1}MB  CPU: {:.1}%",
				state.metrics.frontend_memory_mb, state.metrics.frontend_cpu_percent
			)),
			Span::raw("  |  "),
			Span::styled("Filter: ", Style::default().fg(Color::Cyan)),
			Span::styled(
				format!("{}", state.filter_level),
				Style::default().fg(Color::Yellow),
			),
		]),
	];

	let status_bar =
		Paragraph::new(status_lines).block(Block::default().borders(Borders::ALL).title("Status"));

	frame.render_widget(status_bar, area);
}

/// Draw log panes (backend/frontend)
fn draw_log_panes(frame: &mut Frame, area: Rect, state: &AppState) {
	// Split log area into two panes
	let log_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Percentage(50), // Backend logs
			Constraint::Percentage(50), // Frontend logs
		])
		.split(area);

	// Backend log pane
	draw_log_pane(
		frame,
		log_chunks[0],
		"Backend Logs",
		&state.backend_logs,
		state.active_pane == Pane::Backend,
		state.filter_level,
	);

	// Frontend log pane
	draw_log_pane(
		frame,
		log_chunks[1],
		"Frontend Logs",
		&state.frontend_logs,
		state.active_pane == Pane::Frontend,
		state.filter_level,
	);
}

/// Draw individual log pane
fn draw_log_pane(
	frame: &mut Frame,
	area: Rect,
	title: &str,
	log_buffer: &super::log_buffer::LogBuffer,
	is_active: bool,
	filter_level: super::log_buffer::LogLevel,
) {
	// Collect filtered log lines
	let log_lines: Vec<Line> = log_buffer
		.filtered_entries(filter_level)
		.map(|entry| {
			let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
			Line::from(vec![
				Span::styled(
					format!("[{}] ", timestamp),
					Style::default().fg(Color::DarkGray),
				),
				Span::raw(&entry.message),
			])
		})
		.collect();

	// Active pane has yellow border
	let border_style = if is_active {
		Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD)
	} else {
		Style::default()
	};

	let block = Block::default()
		.borders(Borders::ALL)
		.title(title)
		.border_style(border_style);

	let paragraph = Paragraph::new(log_lines)
		.block(block)
		.wrap(Wrap { trim: false })
		.scroll((log_buffer.scroll_offset() as u16, 0));

	frame.render_widget(paragraph, area);
}

/// Draw control bar
fn draw_control_bar(frame: &mut Frame, area: Rect, _state: &AppState) {
	let controls = vec![Line::from(vec![
		Span::styled("[q]", Style::default().fg(Color::Yellow)),
		Span::raw(" Quit  "),
		Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
		Span::raw(" Switch  "),
		Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
		Span::raw(" Scroll  "),
		Span::styled("[PgUp/PgDn]", Style::default().fg(Color::Yellow)),
		Span::raw(" Page  "),
		Span::styled("[f]", Style::default().fg(Color::Yellow)),
		Span::raw(" Filter  "),
		Span::styled("[c]", Style::default().fg(Color::Yellow)),
		Span::raw(" Clear"),
	])];

	let control_bar = Paragraph::new(controls)
		.block(Block::default().borders(Borders::ALL).title("Controls"))
		.wrap(Wrap { trim: false });

	frame.render_widget(control_bar, area);
}
