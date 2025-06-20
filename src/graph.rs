use crate::Signal;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem},
};
use std::{
    collections::{HashMap, VecDeque},
    io,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

struct GraphData {
    data: VecDeque<(f64, f64)>,
    max_points: usize,
    color: Color,
}

impl GraphData {
    fn new(max_points: usize, color: Color) -> Self {
        Self {
            data: VecDeque::with_capacity(max_points),
            max_points,
            color,
        }
    }

    fn push(&mut self, time: f64, value: f64) {
        if self.data.len() >= self.max_points {
            self.data.pop_front();
        }
        self.data.push_back((time, value));
    }

    fn get_data(&self) -> Vec<(f64, f64)> {
        self.data.iter().cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct GraphMessage {
    pub name: String,
    pub value: f32,
    pub max_points: usize,
}

#[derive(Debug, Clone)]
pub struct DeadlineMessage {
    pub missed: bool,
    pub processing_time_us: u64,
    pub deadline_us: u64,
}

struct GraphRegistry {
    streams: HashMap<String, GraphData>,
    time_start: Instant,
    colors: Vec<Color>,
    next_color: usize,
    deadline_misses: u64,
    total_samples: u64,
    last_processing_time_us: u64,
}

impl GraphRegistry {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            time_start: Instant::now(),
            colors: vec![
                Color::Red,
                Color::Green,
                Color::Blue,
                Color::Yellow,
                Color::Cyan,
                Color::Magenta,
                Color::White,
            ],
            next_color: 0,
            deadline_misses: 0,
            total_samples: 0,
            last_processing_time_us: 0,
        }
    }

    #[allow(clippy::map_entry)]
    fn register_stream(&mut self, name: String, max_points: usize) {
        if !self.streams.contains_key(&name) {
            let color = self.colors[self.next_color % self.colors.len()];
            self.next_color += 1;
            self.streams.insert(name, GraphData::new(max_points, color));
        }
    }

    fn push_value(&mut self, name: &str, value: f32, max_points: usize) {
        self.register_stream(name.to_string(), max_points);
        if let Some(stream) = self.streams.get_mut(name) {
            let time = self.time_start.elapsed().as_secs_f64();
            stream.push(time, value as f64);
        }
    }

    fn get_streams(&self) -> &HashMap<String, GraphData> {
        &self.streams
    }

    fn update_deadline_stats(&mut self, deadline_msg: DeadlineMessage) {
        self.total_samples += 1;
        self.last_processing_time_us = deadline_msg.processing_time_us;
        if deadline_msg.missed {
            self.deadline_misses += 1;
        }
    }

    fn get_deadline_miss_rate(&self) -> f64 {
        if self.total_samples == 0 {
            0.0
        } else {
            (self.deadline_misses as f64 / self.total_samples as f64) * 100.0
        }
    }
}

static GRAPH_SENDER: Mutex<Option<mpsc::Sender<GraphMessage>>> = Mutex::new(None);
static DEADLINE_SENDER: Mutex<Option<mpsc::Sender<DeadlineMessage>>> = Mutex::new(None);

impl Signal {
    pub fn graph(&self, name: &str, value: f32, max_points: usize) {
        let sender = GRAPH_SENDER.lock().unwrap();
        if let Some(ref tx) = *sender {
            let _ = tx.send(GraphMessage {
                name: name.to_string(),
                value,
                max_points,
            });
        }
    }
}

struct GraphApp {
    should_quit: bool,
    last_update: Instant,
    update_interval: Duration,
    registry: GraphRegistry,
    receiver: mpsc::Receiver<GraphMessage>,
    deadline_receiver: mpsc::Receiver<DeadlineMessage>,
}

impl GraphApp {
    fn new(
        receiver: mpsc::Receiver<GraphMessage>,
        deadline_receiver: mpsc::Receiver<DeadlineMessage>,
    ) -> Self {
        Self {
            should_quit: false,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(50),
            registry: GraphRegistry::new(),
            receiver,
            deadline_receiver,
        }
    }

    fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            while let Ok(msg) = self.receiver.try_recv() {
                self.registry
                    .push_value(&msg.name, msg.value, msg.max_points);
            }

            while let Ok(deadline_msg) = self.deadline_receiver.try_recv() {
                self.registry.update_deadline_stats(deadline_msg);
            }

            if self.last_update.elapsed() >= self.update_interval {
                terminal.draw(|f| self.ui(f))?;
                self.last_update = Instant::now();
            }

            if event::poll(Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.should_quit = true;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit {
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }
        ratatui::restore();
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let streams = self.registry.get_streams();

        if streams.is_empty() {
            let block = Block::default().title("Signal Graph").borders(Borders::ALL);
            let text = vec![ListItem::new("No data streams registered")];
            let list = List::new(text).block(block);
            f.render_widget(list, f.area());
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(70),
                Constraint::Percentage(20),
                Constraint::Percentage(10),
            ])
            .split(f.area());

        self.render_chart(f, chunks[0], streams);
        self.render_legend(f, chunks[1], streams);
        self.render_performance(f, chunks[2]);
    }

    #[allow(clippy::type_complexity)]
    fn render_chart(&self, f: &mut Frame, area: Rect, streams: &HashMap<String, GraphData>) {
        let mut all_data: Vec<(String, Vec<(f64, f64)>, Color)> = Vec::new();
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for (name, stream) in streams {
            let data = stream.get_data();
            if !data.is_empty() {
                for (x, y) in &data {
                    min_x = min_x.min(*x);
                    max_x = max_x.max(*x);
                    min_y = min_y.min(*y);
                    max_y = max_y.max(*y);
                }

                all_data.push((name.clone(), data, stream.color));
            }
        }

        let datasets: Vec<Dataset> = all_data
            .iter()
            .map(|(name, data, color)| {
                Dataset::default()
                    .name(Line::from(name.clone()))
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(*color))
                    .graph_type(GraphType::Line)
                    .data(data)
            })
            .collect();

        if datasets.is_empty() {
            return;
        }

        let y_range = if (max_y - min_y).abs() < f64::EPSILON {
            [min_y - 1.0, max_y + 1.0]
        } else {
            let padding = (max_y - min_y) * 0.1;
            [min_y - padding, max_y + padding]
        };

        let x_range = if (max_x - min_x).abs() < f64::EPSILON {
            [min_x - 1.0, max_x + 1.0]
        } else {
            [min_x, max_x]
        };

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title("Signal Graph - Press 'q' to quit")
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("Time (s)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(x_range),
            )
            .y_axis(
                Axis::default()
                    .title("Value")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(y_range),
            );

        f.render_widget(chart, area);
    }

    fn render_legend(&self, f: &mut Frame, area: Rect, streams: &HashMap<String, GraphData>) {
        let items: Vec<ListItem> = streams
            .iter()
            .map(|(name, stream)| {
                let latest_value = stream
                    .data
                    .back()
                    .map(|(_, v)| format!("{:.3}", v))
                    .unwrap_or_else(|| "N/A".to_string());

                ListItem::new(Line::from(vec![
                    Span::styled("● ", Style::default().fg(stream.color)),
                    Span::raw(format!("{}: {}", name, latest_value)),
                ]))
            })
            .collect();

        let legend =
            List::new(items).block(Block::default().title("Data Streams").borders(Borders::ALL));

        f.render_widget(legend, area);
    }

    fn render_performance(&self, f: &mut Frame, area: Rect) {
        let miss_rate = self.registry.get_deadline_miss_rate();
        let processing_time = self.registry.last_processing_time_us;
        let deadline_us = 22; // ~22μs per sample at 44.1kHz

        let performance_color = if miss_rate > 5.0 {
            Color::Red
        } else if miss_rate > 1.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        let items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("● ", Style::default().fg(performance_color)),
                Span::raw(format!("Deadline Miss Rate: {:.2}%", miss_rate)),
            ])),
            ListItem::new(Line::from(vec![Span::raw(format!(
                "Processing Time: {}μs / {}μs",
                processing_time, deadline_us
            ))])),
            ListItem::new(Line::from(vec![Span::raw(format!(
                "Total Samples: {}",
                self.registry.total_samples
            ))])),
        ];

        let performance =
            List::new(items).block(Block::default().title("Performance").borders(Borders::ALL));

        f.render_widget(performance, area);
    }
}

pub fn graph<F>(synth_fn: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&mut Signal) + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let (deadline_tx, deadline_rx) = mpsc::channel();

    {
        let mut sender = GRAPH_SENDER.lock().unwrap();
        *sender = Some(tx);
    }

    {
        let mut deadline_sender = DEADLINE_SENDER.lock().unwrap();
        *deadline_sender = Some(deadline_tx);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let running = Arc::new(Mutex::new(true));

    let audio_handle = {
        let running_clone = running.clone();
        thread::spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
                use std::sync::{Arc, Mutex};

                let host = cpal::default_host();
                let device = host.default_output_device().ok_or("No output device")?;
                let config = device
                    .default_output_config()
                    .expect("Failed to get default output config");
                let channels = config.channels() as usize;

                let signal = Arc::new(Mutex::new(Signal::new(config.sample_rate().0 as usize)));
                let synth_fn = Arc::new(Mutex::new(synth_fn));

                let stream = device.build_output_stream(
                    &config.into(),
                    {
                        let signal = signal.clone();
                        let synth_fn = synth_fn.clone();
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            let deadline_us = 22; // ~22μs per sample at 44.1kHz

                            let mut signal_lock = signal.lock().unwrap();
                            let mut synth_lock = synth_fn.lock().unwrap();

                            for frame in data.chunks_mut(channels) {
                                let start_time = Instant::now();

                                synth_lock(&mut signal_lock);
                                let sample = signal_lock.get_current_sample();

                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = sample;
                                }

                                signal_lock.advance();

                                let processing_time = start_time.elapsed();
                                let processing_time_us = processing_time.as_micros() as u64;
                                let missed = processing_time_us > deadline_us;

                                if let Some(deadline_tx) = DEADLINE_SENDER.lock().unwrap().as_ref()
                                {
                                    let _ = deadline_tx.send(DeadlineMessage {
                                        missed,
                                        processing_time_us,
                                        deadline_us,
                                    });
                                }
                            }
                        }
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None,
                )?;

                stream.play()?;

                while *running_clone.lock().unwrap() {
                    thread::sleep(Duration::from_millis(100));
                }

                Ok(())
            },
        )
    };

    let mut app = GraphApp::new(rx, deadline_rx);
    let res = app.run(&mut terminal);

    *running.lock().unwrap() = false;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = audio_handle.join().unwrap() {
        eprintln!("Audio thread error: {:?}", e);
    }

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
