mod draw;
mod game;
mod scores;

use draw::{Face, FACE_SIZE, HILITE, INK, LED_H, LED_W, NAVY, SHADOW, SILVER};
use game::{Config, Game, Mark, Mode, Status};
use scores::{Leaderboard, Placement, Run};
use macroquad::prelude::*;

const CELL: f32 = 32.0;
const MENU_H: f32 = 24.0;
const PAD: f32 = 12.0;
const BORDER: f32 = 6.0;
const HEADER_H: f32 = 66.0;
const FONT: u16 = 15;

fn window_size(c: Config) -> (f32, f32) {
    let w = 2.0 * PAD + 2.0 * BORDER + c.width as f32 * CELL;
    let h = MENU_H + PAD + HEADER_H + PAD + 2.0 * BORDER + c.height as f32 * CELL + PAD;
    (w, h)
}

fn board_origin() -> Vec2 {
    vec2(PAD + BORDER, MENU_H + PAD + HEADER_H + PAD + BORDER)
}

fn window_conf() -> Conf {
    let (w, h) = window_size(Config::BEGINNER);
    Conf {
        window_title: "Minesweeper".to_owned(),
        window_width: w as i32,
        window_height: h as i32,
        window_resizable: false,
        high_dpi: true,
        icon: app_icon(),
        // Matches StartupWMClass in pkgbuild/minesweeper.desktop, so the launcher's
        // icon and name follow the window.
        platform: miniquad::conf::Platform {
            linux_wm_class: "minesweeper",
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Window / Dock icon, rendered from assets/icon.svg.
fn app_icon() -> Option<miniquad::conf::Icon> {
    fn rgba<const N: usize>(png: &[u8]) -> Option<[u8; N]> {
        let img = Image::from_file_with_format(png, Some(ImageFormat::Png)).ok()?;
        img.bytes.try_into().ok()
    }
    Some(miniquad::conf::Icon {
        small: rgba(include_bytes!("../assets/icon-16.png"))?,
        medium: rgba(include_bytes!("../assets/icon-32.png"))?,
        big: rgba(include_bytes!("../assets/icon-64.png"))?,
    })
}

fn new_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5EED)
}

// Layout is done in logical points; on Retina displays everything is rendered at
// `dpi()` times that resolution through the camera so pixel art and text stay crisp.
fn dpi() -> f32 {
    screen_dpi_scale()
}

/// Window content size in logical points (macroquad already divides out the DPI scale).
fn logical_screen() -> Vec2 {
    vec2(screen_width(), screen_height())
}

/// Mouse position in logical points (macroquad already divides out the DPI scale).
fn mouse() -> Vec2 {
    mouse_position().into()
}

/// UI font: Noto Sans (SIL OFL, see assets/NotoSans-OFL.txt). macroquad's built-in font
/// is a tiny bitmap font that turns blurry and blocky when scaled to menu size.
fn ui_font() -> &'static Font {
    thread_local! {
        static UI_FONT: &'static Font = Box::leak(Box::new(
            load_ttf_font_from_bytes(include_bytes!("../assets/NotoSans-Regular.ttf"))
                .expect("embedded UI font"),
        ));
    }
    UI_FONT.with(|f| *f)
}

fn font_px() -> u16 {
    (FONT as f32 * dpi()).round() as u16
}

fn text(s: &str, x: f32, baseline: f32, color: Color) {
    let params = TextParams {
        font: Some(ui_font()),
        font_size: font_px(),
        font_scale: 1.0 / dpi(),
        color,
        ..Default::default()
    };
    draw_text_ex(s, x, baseline, params);
}

fn text_width(s: &str) -> f32 {
    measure_text(s, Some(ui_font()), font_px(), 1.0 / dpi()).width
}

/// Rasterize every printable ASCII glyph up front. Glyphs added to macroquad's font
/// atlas mid-frame can fail to draw, which showed up as missing letters in the dialog.
fn preload_glyphs() {
    ui_font().populate_font_cache(&Font::ascii_character_list(), font_px());
}

fn button(r: Rect, label: &str, pressed: bool) {
    draw_rectangle(r.x, r.y, r.w, r.h, INK);
    draw::panel(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, !pressed);
    let off = if pressed { 1.0 } else { 0.0 };
    text(label, r.x + (r.w - text_width(label)) / 2.0 + off, r.y + r.h / 2.0 + 6.0 + off, INK);
}

// ---------------------------------------------------------------- menu

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    New,
    Mode(Mode),
    Marks,
    Leaderboard,
    Exit,
    Separator,
}

const MENU_ITEMS: [Item; 11] = [
    Item::New,
    Item::Separator,
    Item::Mode(Mode::Beginner),
    Item::Mode(Mode::Intermediate),
    Item::Mode(Mode::Expert),
    Item::Mode(Mode::Custom),
    Item::Separator,
    Item::Marks,
    Item::Leaderboard,
    Item::Separator,
    Item::Exit,
];
const MENU_TITLE: Rect = Rect { x: 4.0, y: 2.0, w: 56.0, h: MENU_H - 4.0 };
const MENU_W: f32 = 210.0;
const ROW_H: f32 = 22.0;
const SEP_H: f32 = 9.0;

fn item_label(item: Item) -> (&'static str, &'static str) {
    match item {
        Item::New => ("New", "F2"),
        Item::Mode(Mode::Beginner) => ("Beginner", "1"),
        Item::Mode(Mode::Intermediate) => ("Intermediate", "2"),
        Item::Mode(Mode::Expert) => ("Expert", "3"),
        Item::Mode(Mode::Custom) => ("Custom...", "4"),
        Item::Marks => ("Marks (?)", "M"),
        Item::Leaderboard => ("Leaderboard...", "L"),
        Item::Exit => ("Exit", "Q"),
        Item::Separator => ("", ""),
    }
}

fn menu_rect() -> Rect {
    let h: f32 = MENU_ITEMS
        .iter()
        .map(|&i| if i == Item::Separator { SEP_H } else { ROW_H })
        .sum();
    Rect::new(MENU_TITLE.x, MENU_H - 1.0, MENU_W, h + 6.0)
}

fn menu_layout() -> Vec<(Item, Rect)> {
    let m = menu_rect();
    let mut y = m.y + 3.0;
    MENU_ITEMS
        .iter()
        .map(|&item| {
            let h = if item == Item::Separator { SEP_H } else { ROW_H };
            let r = Rect::new(m.x + 3.0, y, m.w - 6.0, h);
            y += h;
            (item, r)
        })
        .collect()
}

// ---------------------------------------------------------------- dialogs

enum Dialog {
    Custom(CustomDialog),
    Leaderboard(LeaderboardDialog),
    Summary(SummaryDialog),
}

/// What a dialog asks the app to do once it's dismissed.
enum Action {
    Close,
    Configure(Config),
    NewGame,
    ShowLeaderboard(Mode),
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Beginner => "Beginner",
        Mode::Intermediate => "Intermediate",
        Mode::Expert => "Expert",
        Mode::Custom => "Custom",
    }
}

/// A `w` x `h` dialog centered in the window, never over the menu bar.
fn centered(w: f32, h: f32) -> Rect {
    let Vec2 { x: sw, y: sh } = logical_screen();
    Rect::new(((sw - w) / 2.0).round(), ((sh - h) / 2.0).round().max(MENU_H), w, h)
}

/// Outline, raised face and navy title bar shared by every dialog.
fn dialog_frame(r: Rect, title: &str) {
    draw_rectangle(r.x, r.y, r.w, r.h, INK);
    draw::panel(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, true);
    draw_rectangle(r.x + 4.0, r.y + 4.0, r.w - 8.0, 22.0, NAVY);
    text(title, r.x + 10.0, r.y + 20.0, HILITE);
}

fn clicked(r: Rect, mouse: Vec2) -> bool {
    is_mouse_button_released(MouseButton::Left) && r.contains(mouse)
}

// ---------------------------------------------------------------- custom dialog

const DIALOG_W: f32 = 264.0;
const DIALOG_H: f32 = 200.0;
const FIELD_LABELS: [&str; 3] = ["Height:", "Width:", "Mines:"];

struct Repeat {
    field: usize,
    delta: i32,
    next_at: f64,
}

struct CustomDialog {
    values: [usize; 3], // height, width, mines
    repeat: Option<Repeat>,
}

impl CustomDialog {
    fn new(c: Config) -> Self {
        CustomDialog { values: [c.height, c.width, c.mines], repeat: None }
    }

    fn rect(&self) -> Rect {
        centered(DIALOG_W, DIALOG_H)
    }

    fn row_y(&self, field: usize) -> f32 {
        self.rect().y + 38.0 + field as f32 * 34.0
    }

    fn minus_rect(&self, field: usize) -> Rect {
        Rect::new(self.rect().x + 172.0, self.row_y(field), 26.0, 26.0)
    }

    fn plus_rect(&self, field: usize) -> Rect {
        Rect::new(self.rect().x + 202.0, self.row_y(field), 26.0, 26.0)
    }

    fn ok_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + 36.0, r.y + r.h - 46.0, 86.0, 30.0)
    }

    fn cancel_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + r.w - 122.0, r.y + r.h - 46.0, 86.0, 30.0)
    }

    fn config(&self) -> Config {
        Config::custom(self.values[1], self.values[0], self.values[2])
    }

    fn step(&mut self, field: usize, delta: i32) {
        let v = (self.values[field] as i32 + delta).max(0) as usize;
        self.values[field] = v;
        let c = Config::custom(self.values[1], self.values[0], self.values[2]);
        self.values = [c.height, c.width, c.mines];
    }

    fn update(&mut self, mouse: Vec2) -> Option<Action> {
        if is_key_pressed(KeyCode::Escape) {
            return Some(Action::Close);
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            return Some(Action::Configure(self.config()));
        }

        let now = get_time();
        let big = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let amount = if big { 10 } else { 1 };

        if is_mouse_button_pressed(MouseButton::Left) {
            for field in 0..3 {
                for (r, sign) in [(self.minus_rect(field), -1), (self.plus_rect(field), 1)] {
                    if r.contains(mouse) {
                        self.step(field, sign * amount);
                        self.repeat = Some(Repeat { field, delta: sign, next_at: now + 0.4 });
                    }
                }
            }
        }
        if let Some(Repeat { field, delta, next_at }) = self.repeat
            && is_mouse_button_down(MouseButton::Left)
        {
            let r = if delta < 0 { self.minus_rect(field) } else { self.plus_rect(field) };
            if r.contains(mouse) && now >= next_at {
                self.repeat = Some(Repeat { field, delta, next_at: now + 0.035 });
                self.step(field, delta * amount);
            }
        }
        if is_mouse_button_released(MouseButton::Left) {
            self.repeat = None;
            if self.ok_rect().contains(mouse) {
                return Some(Action::Configure(self.config()));
            }
            if self.cancel_rect().contains(mouse) {
                return Some(Action::Close);
            }
        }
        None
    }

    fn draw(&self, mouse: Vec2) {
        let r = self.rect();
        dialog_frame(r, "Custom Field");

        let held = is_mouse_button_down(MouseButton::Left);
        for (field, label) in FIELD_LABELS.iter().enumerate() {
            let y = self.row_y(field);
            text(label, r.x + 18.0, y + 19.0, INK);
            let vb = Rect::new(r.x + 96.0, y, 66.0, 26.0);
            draw_rectangle(vb.x, vb.y, vb.w, vb.h, HILITE);
            draw::bevel(vb.x, vb.y, vb.w, vb.h, 2.0, false);
            let v = self.values[field].to_string();
            text(&v, vb.x + vb.w - 8.0 - text_width(&v), y + 19.0, INK);
            let (mr, pr) = (self.minus_rect(field), self.plus_rect(field));
            button(mr, "-", held && mr.contains(mouse));
            button(pr, "+", held && pr.contains(mouse));
        }
        let (ok, cancel) = (self.ok_rect(), self.cancel_rect());
        button(ok, "OK", held && ok.contains(mouse));
        button(cancel, "Cancel", held && cancel.contains(mouse));
    }
}

// ---------------------------------------------------------------- leaderboard dialog

const BOARD_W: f32 = 300.0;
const BOARD_H: f32 = 352.0;
const BOARD_ROW_H: f32 = 20.0;
/// Sized to their labels; together they span the list below.
const TAB_W: [f32; 3] = [84.0, 108.0, 76.0];

/// The ten fastest wins for each preset, one difficulty at a time behind tabs.
struct LeaderboardDialog {
    tab: usize,
    tops: [Vec<Run>; 3],
    totals: [usize; 3],
    /// The run just played, highlighted if it made the list.
    highlight: Option<Run>,
}

impl LeaderboardDialog {
    fn new(board: &Leaderboard, mode: Mode, highlight: Option<Run>) -> Self {
        LeaderboardDialog {
            tab: scores::RANKED.iter().position(|&m| m == mode).unwrap_or(0),
            tops: scores::RANKED.map(|m| board.top(m)),
            totals: scores::RANKED.map(|m| board.ranked(m).len()),
            highlight,
        }
    }

    fn rect(&self) -> Rect {
        centered(BOARD_W, BOARD_H)
    }

    fn tab_rect(&self, i: usize) -> Rect {
        let r = self.rect();
        let x = r.x + 12.0 + TAB_W[..i].iter().map(|w| w + 4.0).sum::<f32>();
        Rect::new(x, r.y + 36.0, TAB_W[i], 26.0)
    }

    fn list_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + 12.0, r.y + 70.0, r.w - 24.0, 230.0)
    }

    fn close_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + r.w - 98.0, r.y + r.h - 44.0, 86.0, 30.0)
    }

    fn update(&mut self, mouse: Vec2) -> Option<Action> {
        if [KeyCode::Escape, KeyCode::Enter, KeyCode::KpEnter, KeyCode::L].into_iter().any(is_key_pressed) {
            return Some(Action::Close);
        }
        if is_key_pressed(KeyCode::Left) {
            self.tab = (self.tab + 2) % 3;
        }
        if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Tab) {
            self.tab = (self.tab + 1) % 3;
        }
        for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].into_iter().enumerate() {
            if is_key_pressed(key) {
                self.tab = i;
            }
        }
        if is_mouse_button_pressed(MouseButton::Left)
            && let Some(i) = (0..3).find(|&i| self.tab_rect(i).contains(mouse))
        {
            self.tab = i;
        }
        clicked(self.close_rect(), mouse).then_some(Action::Close)
    }

    fn draw(&self, mouse: Vec2) {
        let r = self.rect();
        dialog_frame(r, "Leaderboard");
        for (i, &mode) in scores::RANKED.iter().enumerate() {
            button(self.tab_rect(i), mode_name(mode), i == self.tab);
        }

        let lb = self.list_rect();
        draw_rectangle(lb.x, lb.y, lb.w, lb.h, HILITE);
        draw::bevel(lb.x, lb.y, lb.w, lb.h, 2.0, false);
        let (rank_right, time_right, date_x) = (lb.x + 36.0, lb.x + 130.0, lb.x + 158.0);
        let header = lb.y + 19.0;
        text("#", rank_right - text_width("#"), header, SHADOW);
        text("Seconds", time_right - text_width("Seconds"), header, SHADOW);
        text("Date", date_x, header, SHADOW);
        draw_rectangle(lb.x + 6.0, lb.y + 25.0, lb.w - 12.0, 1.0, SHADOW);

        let top = &self.tops[self.tab];
        if top.is_empty() {
            let msg = "No wins yet.";
            text(msg, lb.x + (lb.w - text_width(msg)) / 2.0, lb.y + lb.h / 2.0 + 6.0, SHADOW);
        }
        for (i, run) in top.iter().enumerate() {
            let y = lb.y + 28.0 + i as f32 * BOARD_ROW_H;
            let mine = self.highlight == Some(*run);
            if mine {
                draw_rectangle(lb.x + 3.0, y, lb.w - 6.0, BOARD_ROW_H, NAVY);
            }
            let color = if mine { HILITE } else { INK };
            let (rank, time) = (format!("{}.", i + 1), scores::format_time(run.millis));
            text(&rank, rank_right - text_width(&rank), y + 15.0, color);
            text(&time, time_right - text_width(&time), y + 15.0, color);
            text(&scores::format_date(run.when), date_x, y + 15.0, color);
        }

        let close = self.close_rect();
        let total = match self.totals[self.tab] {
            1 => "1 win".to_owned(),
            n => format!("{n} wins"),
        };
        text(&total, r.x + 16.0, close.y + 21.0, INK);
        button(close, "Close", is_mouse_button_down(MouseButton::Left) && close.contains(mouse));
    }
}

// ---------------------------------------------------------------- end-of-game summary

const SUMMARY_W: f32 = 280.0;
const SUMMARY_H: f32 = 210.0;
/// Pause before the summary pops up, so the finished board (or the explosion) shows first.
const SUMMARY_DELAY: f64 = 0.6;

struct SummaryDialog {
    mode: Mode,
    won: bool,
    millis: u64,
    cleared: f32,
    /// Where the run placed; only wins on a preset difficulty are ranked.
    placement: Option<Placement>,
    saved: bool,
}

impl SummaryDialog {
    fn rect(&self) -> Rect {
        centered(SUMMARY_W, SUMMARY_H)
    }

    fn new_game_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + 36.0, r.y + r.h - 46.0, 92.0, 30.0)
    }

    fn close_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + r.w - 128.0, r.y + r.h - 46.0, 92.0, 30.0)
    }

    fn update(&mut self, mouse: Vec2) -> Option<Action> {
        if [KeyCode::Enter, KeyCode::KpEnter, KeyCode::F2, KeyCode::N].into_iter().any(is_key_pressed) {
            return Some(Action::NewGame);
        }
        if is_key_pressed(KeyCode::Escape) {
            return Some(Action::Close);
        }
        if is_key_pressed(KeyCode::L) {
            return Some(Action::ShowLeaderboard(self.mode));
        }
        if clicked(self.new_game_rect(), mouse) {
            return Some(Action::NewGame);
        }
        clicked(self.close_rect(), mouse).then_some(Action::Close)
    }

    fn rows(&self) -> Vec<(&'static str, String)> {
        let mut rows = vec![
            ("Difficulty", mode_name(self.mode).to_owned()),
            ("Time", format!("{} s", scores::format_time(self.millis))),
        ];
        match self.placement {
            Some(p) => {
                rows.push(("Place", format!("{} of {}", scores::ordinal(p.rank), p.total)));
                rows.push(("Percentile", scores::ordinal(p.percentile as usize)));
            }
            None if !self.won => rows.push(("Cleared", format!("{:.0}%", (self.cleared * 100.0).floor()))),
            None => {}
        }
        rows
    }

    fn note(&self) -> &'static str {
        match self.placement {
            _ if !self.saved => "Couldn't save this run.",
            None if !self.won => "Only wins go on the leaderboard.",
            None => "Custom games aren't ranked.",
            Some(p) if p.total == 1 => "Your first win!",
            Some(p) if p.rank == 1 => "New best time!",
            Some(p) if p.rank <= scores::TOP => "You made the top ten!",
            Some(_) => "",
        }
    }

    fn draw(&self, mouse: Vec2) {
        let r = self.rect();
        dialog_frame(r, if self.won { "You won!" } else { "Game over" });
        for (i, (label, value)) in self.rows().iter().enumerate() {
            let y = r.y + 54.0 + i as f32 * 24.0;
            text(label, r.x + 24.0, y, INK);
            text(value, r.x + 124.0, y, INK);
        }
        let note = self.note();
        text(note, r.x + (r.w - text_width(note)) / 2.0, r.y + 152.0, NAVY);

        let held = is_mouse_button_down(MouseButton::Left);
        let (new_game, close) = (self.new_game_rect(), self.close_rect());
        button(new_game, "New Game", held && new_game.contains(mouse));
        button(close, "Close", held && close.contains(mouse));
    }
}

// ---------------------------------------------------------------- app

struct App {
    mode: Mode,
    config: Config,
    game: Game,
    marks: bool,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    menu_open: bool,
    dialog: Option<Dialog>,
    leaderboard: Leaderboard,
    /// The latest recorded win, highlighted on the leaderboard.
    last_run: Option<Run>,
    /// The end-of-game summary and when to show it.
    pending_summary: Option<(f64, SummaryDialog)>,
    face_down: bool,
    /// Left button went down on the board and hasn't been released yet.
    left_board: bool,
    /// Both buttons (or middle) are down: releasing will chord.
    chording: bool,
    /// After a chord, ignore further releases until every button is up.
    swallow: bool,
    quit: bool,
    window: WindowFit,
}

/// Keeps the window's content area at the size the board needs.
///
/// We call miniquad directly: macroquad's `request_new_screen_size` multiplies by the DPI
/// scale, but on macOS miniquad applies the value in points, doubling the window on Retina.
/// miniquad also sizes the whole window frame, so the content ends up short by the
/// title-bar height. Once the size settles we measure that error, re-request, and remember
/// the offset so later mode switches land in one step.
///
/// A large error isn't the title bar, it's the window manager imposing a size (e.g. an
/// Omarchy floating workspace opens every window at up to 1100x760). Folding that into
/// the offset would drive the request to zero, which X rejects with BadValue, so we just
/// ask for the board's size again instead.
struct WindowFit {
    desired: Vec2,
    offset: Vec2,
    last: Vec2,
    stable_frames: u32,
    fixes: u32,
}

/// Largest content-size error we attribute to window decorations, in logical points.
const MAX_FRAME_ERROR: f32 = 64.0;

impl WindowFit {
    fn new(desired: Vec2) -> Self {
        WindowFit { desired, offset: Vec2::ZERO, last: Vec2::ZERO, stable_frames: 0, fixes: 0 }
    }

    fn request(&mut self, desired: Vec2) {
        self.desired = desired;
        self.stable_frames = 0;
        self.fixes = 0;
        self.apply();
    }

    fn apply(&self) {
        let target = (self.desired + self.offset).max(self.desired * 0.5);
        miniquad::window::set_window_size(target.x.round() as u32, target.y.round() as u32);
    }

    fn update(&mut self) {
        let current = logical_screen();
        if current.distance(self.last) < 0.5 {
            self.stable_frames += 1;
        } else {
            self.stable_frames = 0;
            self.last = current;
        }
        let err = self.desired - current;
        if self.stable_frames == 8 && self.fixes < 3 && (err.x.abs() > 1.5 || err.y.abs() > 1.5) {
            if err.x.abs() <= MAX_FRAME_ERROR && err.y.abs() <= MAX_FRAME_ERROR {
                self.offset += err;
            }
            self.fixes += 1;
            self.apply();
        }
    }
}

impl App {
    fn new() -> Self {
        let config = Config::BEGINNER;
        App {
            mode: Mode::Beginner,
            config,
            game: Game::new(config, new_seed()),
            marks: true,
            started_at: None,
            finished_at: None,
            menu_open: false,
            dialog: None,
            leaderboard: Leaderboard::load(),
            last_run: None,
            pending_summary: None,
            face_down: false,
            left_board: false,
            chording: false,
            swallow: false,
            quit: false,
            window: WindowFit::new(window_size(config).into()),
        }
    }

    fn new_game(&mut self) {
        self.game = Game::new(self.config, new_seed());
        self.started_at = None;
        self.finished_at = None;
        self.pending_summary = None;
        self.left_board = false;
        self.chording = false;
        self.swallow = true;
    }

    fn set_config(&mut self, mode: Mode, config: Config) {
        self.mode = mode;
        self.config = config;
        self.window.request(window_size(config).into());
        self.new_game();
    }

    fn activate(&mut self, item: Item) {
        match item {
            Item::New => self.new_game(),
            Item::Mode(Mode::Custom) => self.dialog = Some(Dialog::Custom(CustomDialog::new(self.config))),
            Item::Mode(m) => self.set_config(m, m.preset().unwrap()),
            Item::Marks => self.marks = !self.marks,
            Item::Leaderboard => self.show_leaderboard(self.mode),
            Item::Exit => self.quit = true,
            Item::Separator => {}
        }
    }

    fn show_leaderboard(&mut self, mode: Mode) {
        let highlight = self.last_run.filter(|r| r.mode == mode);
        self.dialog = Some(Dialog::Leaderboard(LeaderboardDialog::new(&self.leaderboard, mode, highlight)));
    }

    fn perform(&mut self, action: Action) {
        self.dialog = None;
        match action {
            Action::Close => {}
            Action::Configure(cfg) => self.set_config(Mode::Custom, cfg),
            Action::NewGame => self.new_game(),
            Action::ShowLeaderboard(mode) => self.show_leaderboard(mode),
        }
    }

    /// Records a finished game and queues its summary.
    fn finish(&mut self, now: f64) {
        let start = self.started_at.unwrap_or(now);
        let millis = ((now - start) * 1000.0).round() as u64;
        let won = self.game.status() == Status::Won;
        let (mut placement, mut saved) = (None, true);
        if won && scores::mode_key(self.mode).is_some() {
            let when = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let run = Run { mode: self.mode, millis, when };
            let (p, result) = self.leaderboard.record(run);
            if let Err(e) = result {
                eprintln!("minesweeper: couldn't save the leaderboard: {e}");
                saved = false;
            }
            placement = Some(p);
            self.last_run = Some(run);
        }
        let summary =
            SummaryDialog { mode: self.mode, won, millis, cleared: self.game.cleared(), placement, saved };
        self.pending_summary = Some((now + SUMMARY_DELAY, summary));
    }

    fn elapsed(&self) -> i32 {
        match self.started_at {
            None => 0,
            Some(start) => {
                let end = self.finished_at.unwrap_or_else(get_time);
                ((end - start) as i32 + 1).min(999)
            }
        }
    }

    fn face_rect(&self) -> Rect {
        let (w, _) = window_size(self.config);
        Rect::new(
            ((w - FACE_SIZE) / 2.0).round(),
            MENU_H + PAD + ((HEADER_H - FACE_SIZE) / 2.0).round(),
            FACE_SIZE,
            FACE_SIZE,
        )
    }

    fn cell_at(&self, p: Vec2) -> Option<(usize, usize)> {
        let rel = p - board_origin();
        if rel.x < 0.0 || rel.y < 0.0 {
            return None;
        }
        let (x, y) = ((rel.x / CELL) as usize, (rel.y / CELL) as usize);
        (x < self.config.width && y < self.config.height).then_some((x, y))
    }

    // ------------------------------------------------------------ input

    fn update(&mut self) {
        let mouse = mouse();
        self.window.update();

        if self.dialog.is_none()
            && let Some((at, _)) = self.pending_summary
            && get_time() >= at
            && let Some((_, summary)) = self.pending_summary.take()
        {
            self.menu_open = false;
            self.dialog = Some(Dialog::Summary(summary));
        }

        if let Some(dialog) = &mut self.dialog {
            let action = match dialog {
                Dialog::Custom(d) => d.update(mouse),
                Dialog::Leaderboard(d) => d.update(mouse),
                Dialog::Summary(d) => d.update(mouse),
            };
            if let Some(action) = action {
                self.perform(action);
            }
            return;
        }

        self.handle_keys();
        if self.handle_menu(mouse) {
            return;
        }
        self.handle_face(mouse);
        self.handle_board(mouse);
        self.track_time();
    }

    fn handle_keys(&mut self) {
        let keys = [
            (KeyCode::F2, Item::New),
            (KeyCode::N, Item::New),
            (KeyCode::Key1, Item::Mode(Mode::Beginner)),
            (KeyCode::Key2, Item::Mode(Mode::Intermediate)),
            (KeyCode::Key3, Item::Mode(Mode::Expert)),
            (KeyCode::Key4, Item::Mode(Mode::Custom)),
            (KeyCode::C, Item::Mode(Mode::Custom)),
            (KeyCode::M, Item::Marks),
            (KeyCode::L, Item::Leaderboard),
            (KeyCode::Q, Item::Exit),
        ];
        for (key, item) in keys {
            if is_key_pressed(key) {
                self.menu_open = false;
                self.activate(item);
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            self.menu_open = false;
        }
    }

    /// Returns true when the menu consumed this frame's mouse input.
    fn handle_menu(&mut self, mouse: Vec2) -> bool {
        if is_mouse_button_pressed(MouseButton::Left) {
            if MENU_TITLE.contains(mouse) {
                self.menu_open = !self.menu_open;
                return true;
            }
            if self.menu_open {
                self.menu_open = false;
                self.swallow = true;
                if let Some((item, _)) = menu_layout().into_iter().find(|(_, r)| r.contains(mouse)) {
                    self.activate(item);
                }
                return true;
            }
        }
        if self.menu_open && is_mouse_button_pressed(MouseButton::Right) {
            self.menu_open = false;
            self.swallow = true;
            return true;
        }
        self.menu_open
    }

    fn handle_face(&mut self, mouse: Vec2) {
        let r = self.face_rect();
        if is_mouse_button_pressed(MouseButton::Left) && r.contains(mouse) {
            self.face_down = true;
        }
        if is_mouse_button_released(MouseButton::Left) {
            if self.face_down && r.contains(mouse) {
                self.new_game();
            }
            self.face_down = false;
        }
    }

    fn handle_board(&mut self, mouse: Vec2) {
        use MouseButton::{Left, Middle, Right};
        let (l_down, r_down, m_down) =
            (is_mouse_button_down(Left), is_mouse_button_down(Right), is_mouse_button_down(Middle));

        if !self.game.is_over() {
            let cell = self.cell_at(mouse);

            let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            if is_mouse_button_pressed(Left)
                && !self.swallow
                && let Some((x, y)) = cell
            {
                if shift && !r_down {
                    // Shift+click marks like a right click (handy on trackpads). Swallow the
                    // rest of the press so releasing can never reveal the cell.
                    self.game.toggle_mark(x, y, self.marks);
                    self.swallow = true;
                } else {
                    self.left_board = true;
                    if r_down {
                        self.chording = true;
                    }
                }
            }
            if is_mouse_button_pressed(Right) && !self.swallow {
                if self.left_board && l_down {
                    self.chording = true;
                } else if let Some((x, y)) = cell {
                    self.game.toggle_mark(x, y, self.marks);
                }
            }
            if is_mouse_button_pressed(Middle) && cell.is_some() && !self.swallow {
                self.chording = true;
            }

            let released = is_mouse_button_released(Left)
                || is_mouse_button_released(Right)
                || is_mouse_button_released(Middle);
            if released && self.chording {
                if let Some((x, y)) = cell {
                    self.game.chord(x, y);
                }
                self.chording = false;
                self.left_board = false;
                self.swallow = true;
            } else if is_mouse_button_released(Left) && self.left_board {
                if let Some((x, y)) = cell {
                    if self.game.cell(x, y).revealed {
                        self.game.chord(x, y);
                    } else {
                        self.game.reveal(x, y);
                    }
                }
                self.left_board = false;
            }
        }

        if !l_down && !r_down && !m_down {
            self.swallow = false;
            self.left_board = false;
            self.chording = false;
        }
    }

    fn track_time(&mut self) {
        let now = get_time();
        match self.game.status() {
            Status::Playing if self.started_at.is_none() => self.started_at = Some(now),
            Status::Won | Status::Lost if self.finished_at.is_none() => {
                self.started_at.get_or_insert(now);
                self.finished_at = Some(now);
                self.finish(now);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ rendering

    fn interacting(&self) -> bool {
        !self.game.is_over() && !self.swallow && !self.menu_open && self.dialog.is_none()
    }

    /// Cells drawn sunken while the mouse is held (single press or chord preview).
    fn pressed_cells(&self, mouse: Vec2) -> Vec<(usize, usize)> {
        if !self.interacting() {
            return vec![];
        }
        let Some((x, y)) = self.cell_at(mouse) else { return vec![] };
        let left = self.left_board && is_mouse_button_down(MouseButton::Left);
        if self.chording || (left && self.game.cell(x, y).revealed) {
            std::iter::once((x, y)).chain(self.game.neighbors(x, y)).collect()
        } else if left {
            vec![(x, y)]
        } else {
            vec![]
        }
    }

    fn face(&self) -> Face {
        match self.game.status() {
            Status::Lost => Face::Dead,
            Status::Won => Face::Cool,
            _ if self.interacting() && (self.chording || (self.left_board && is_mouse_button_down(MouseButton::Left))) => {
                Face::Surprised
            }
            _ => Face::Smile,
        }
    }

    fn draw(&self) {
        let mouse = mouse();
        let (w, h) = window_size(self.config);
        let screen = logical_screen();
        // Negative height: from_display_rect is y-up (meant for render targets), we want y-down.
        set_camera(&Camera2D::from_display_rect(Rect::new(0.0, screen.y, screen.x, -screen.y)));
        clear_background(SILVER);

        // Window frame and header.
        draw::bevel(0.0, MENU_H, w, h - MENU_H, 3.0, true);
        draw::bevel(PAD, MENU_H + PAD, w - 2.0 * PAD, HEADER_H, 3.0, false);
        let led_y = MENU_H + PAD + ((HEADER_H - LED_H) / 2.0).round();
        draw::led(self.game.mines_left(), PAD + 10.0, led_y);
        draw::led(self.elapsed(), w - PAD - 10.0 - LED_W, led_y);
        let fr = self.face_rect();
        draw::face_button(fr.x, fr.y, self.face(), self.face_down && fr.contains(mouse));

        // Board.
        let origin = board_origin();
        let (bw, bh) = (self.config.width as f32 * CELL, self.config.height as f32 * CELL);
        draw::bevel(origin.x - BORDER, origin.y - BORDER, bw + 2.0 * BORDER, bh + 2.0 * BORDER, BORDER, false);
        let pressed = self.pressed_cells(mouse);
        for y in 0..self.config.height {
            for x in 0..self.config.width {
                let p = origin + vec2(x as f32, y as f32) * CELL;
                self.draw_cell(x, y, p.x, p.y, pressed.contains(&(x, y)));
            }
        }

        self.draw_menu(mouse, w);
        match &self.dialog {
            Some(Dialog::Custom(d)) => d.draw(mouse),
            Some(Dialog::Leaderboard(d)) => d.draw(mouse),
            Some(Dialog::Summary(d)) => d.draw(mouse),
            None => {}
        }
    }

    fn draw_cell(&self, x: usize, y: usize, px: f32, py: f32, pressed: bool) {
        let c = self.game.cell(x, y);
        let lost = self.game.status() == Status::Lost;

        let flat = |fill: Color| {
            draw_rectangle(px, py, CELL, CELL, fill);
            draw_rectangle(px, py, CELL, 2.0, SHADOW);
            draw_rectangle(px, py, 2.0, CELL, SHADOW);
        };
        let raised = || draw::panel(px, py, CELL, CELL, 4.0, true);

        if c.revealed {
            flat(SILVER);
            if c.adjacent > 0 {
                draw::number(c.adjacent, px, py, CELL);
            }
        } else if lost && c.mine && c.mark != Mark::Flag {
            let boom = self.game.exploded() == Some((x, y));
            flat(if boom { draw::ALARM } else { SILVER });
            draw::mine(px, py, CELL);
        } else if lost && !c.mine && c.mark == Mark::Flag {
            flat(SILVER);
            draw::wrong_flag(px, py, CELL);
        } else {
            match c.mark {
                Mark::Flag => {
                    raised();
                    draw::flag(px, py, CELL);
                }
                Mark::Question if pressed => {
                    flat(SILVER);
                    draw::question(px + 1.0, py + 1.0, CELL);
                }
                Mark::Question => {
                    raised();
                    draw::question(px, py, CELL);
                }
                Mark::None if pressed => flat(SILVER),
                Mark::None => raised(),
            }
        }
    }

    fn draw_menu(&self, mouse: Vec2, w: f32) {
        draw_rectangle(0.0, 0.0, w, MENU_H, SILVER);
        let t = MENU_TITLE;
        if self.menu_open {
            draw_rectangle(t.x, t.y, t.w, t.h, NAVY);
        }
        text("Game", t.x + 8.0, t.y + 15.0, if self.menu_open { HILITE } else { INK });
        if !self.menu_open {
            return;
        }

        let m = menu_rect();
        draw_rectangle(m.x, m.y, m.w, m.h, INK);
        draw::panel(m.x, m.y, m.w - 1.0, m.h - 1.0, 2.0, true);
        for (item, r) in menu_layout() {
            if item == Item::Separator {
                let ly = r.y + (r.h / 2.0).floor();
                draw_rectangle(r.x + 2.0, ly, r.w - 4.0, 1.0, SHADOW);
                draw_rectangle(r.x + 2.0, ly + 1.0, r.w - 4.0, 1.0, HILITE);
                continue;
            }
            let hover = r.contains(mouse);
            if hover {
                draw_rectangle(r.x, r.y, r.w, r.h, NAVY);
            }
            let color = if hover { HILITE } else { INK };
            let checked = match item {
                Item::Mode(m) => m == self.mode,
                Item::Marks => self.marks,
                _ => false,
            };
            if checked {
                draw::checkmark(r.x + 8.0, r.y + 6.0, color);
            }
            let (label, shortcut) = item_label(item);
            text(label, r.x + 26.0, r.y + 16.0, color);
            text(shortcut, r.x + r.w - 12.0 - text_width(shortcut), r.y + 16.0, color);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    preload_glyphs();
    loop {
        app.update();
        if app.quit {
            break;
        }
        app.draw();
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_icon_pngs_decode_at_the_right_sizes() {
        assert!(super::app_icon().is_some());
    }
}
