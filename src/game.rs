//! Core Minesweeper rules, independent of any rendering or input handling.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub width: usize,
    pub height: usize,
    pub mines: usize,
}

impl Config {
    pub const BEGINNER: Config = Config { width: 9, height: 9, mines: 10 };
    pub const INTERMEDIATE: Config = Config { width: 16, height: 16, mines: 40 };
    pub const EXPERT: Config = Config { width: 30, height: 16, mines: 99 };

    pub const MIN_WIDTH: usize = 9;
    pub const MAX_WIDTH: usize = 30;
    pub const MIN_HEIGHT: usize = 9;
    pub const MAX_HEIGHT: usize = 24;
    pub const MIN_MINES: usize = 10;

    pub fn max_mines(width: usize, height: usize) -> usize {
        (width - 1) * (height - 1)
    }

    /// Builds a custom field, clamping every value into the classic legal range.
    pub fn custom(width: usize, height: usize, mines: usize) -> Config {
        let width = width.clamp(Self::MIN_WIDTH, Self::MAX_WIDTH);
        let height = height.clamp(Self::MIN_HEIGHT, Self::MAX_HEIGHT);
        let mines = mines.clamp(Self::MIN_MINES, Self::max_mines(width, height));
        Config { width, height, mines }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Beginner,
    Intermediate,
    Expert,
    Custom,
}

impl Mode {
    pub fn preset(self) -> Option<Config> {
        match self {
            Mode::Beginner => Some(Config::BEGINNER),
            Mode::Intermediate => Some(Config::INTERMEDIATE),
            Mode::Expert => Some(Config::EXPERT),
            Mode::Custom => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    Flag,
    Question,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    pub mine: bool,
    pub adjacent: u8,
    pub revealed: bool,
    pub mark: Mark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// No cell revealed yet; mines are placed on the first reveal.
    Ready,
    Playing,
    Won,
    Lost,
}

/// The up-to-8 in-bounds neighbours of `(x, y)` on a `width` x `height` grid.
pub fn neighbors(
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let (w, h, x, y) = (width as isize, height as isize, x as isize, y as isize);
    (-1..=1)
        .flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
        .filter(move |&(nx, ny)| (nx, ny) != (x, y) && nx >= 0 && ny >= 0 && nx < w && ny < h)
        .map(|(nx, ny)| (nx as usize, ny as usize))
}

pub struct Game {
    config: Config,
    cells: Vec<Cell>,
    status: Status,
    exploded: Option<(usize, usize)>,
    revealed: usize,
    flags: usize,
    rng: Rng,
}

impl Game {
    pub fn new(config: Config, seed: u64) -> Self {
        Game {
            config,
            cells: vec![Cell::default(); config.width * config.height],
            status: Status::Ready,
            exploded: None,
            revealed: 0,
            flags: 0,
            rng: Rng::new(seed),
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_over(&self) -> bool {
        matches!(self.status, Status::Won | Status::Lost)
    }

    /// The mine that was clicked to lose the game, if any.
    pub fn exploded(&self) -> Option<(usize, usize)> {
        self.exploded
    }

    /// Mines minus flags placed. Goes negative if the player over-flags, like the original.
    pub fn mines_left(&self) -> i32 {
        self.config.mines as i32 - self.flags as i32
    }

    /// Share of the safe cells uncovered so far, 0.0-1.0.
    pub fn cleared(&self) -> f32 {
        let safe = self.config.width * self.config.height - self.config.mines;
        self.revealed as f32 / safe as f32
    }

    pub fn cell(&self, x: usize, y: usize) -> &Cell {
        &self.cells[self.idx(x, y)]
    }

    pub fn neighbors(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
        neighbors(self.config.width, self.config.height, x, y)
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.config.width + x
    }

    /// Left click: open a cell. The first reveal of a game is always safe.
    pub fn reveal(&mut self, x: usize, y: usize) {
        if self.is_over() {
            return;
        }
        let i = self.idx(x, y);
        if self.cells[i].revealed || self.cells[i].mark == Mark::Flag {
            return;
        }
        if self.status == Status::Ready {
            self.place_mines(x, y);
            self.status = Status::Playing;
        }
        if self.cells[i].mine {
            self.lose(x, y);
            return;
        }
        self.flood(x, y);
        self.check_win();
    }

    /// Chord: on a revealed number whose flag count matches, open every other neighbour.
    pub fn chord(&mut self, x: usize, y: usize) {
        if self.status != Status::Playing {
            return;
        }
        let cell = *self.cell(x, y);
        if !cell.revealed || cell.adjacent == 0 {
            return;
        }
        let flagged = self
            .neighbors(x, y)
            .filter(|&(nx, ny)| self.cell(nx, ny).mark == Mark::Flag)
            .count();
        if flagged != cell.adjacent as usize {
            return;
        }
        let targets: Vec<_> = self
            .neighbors(x, y)
            .filter(|&(nx, ny)| {
                let c = self.cell(nx, ny);
                !c.revealed && c.mark != Mark::Flag
            })
            .collect();
        if let Some(&(mx, my)) = targets.iter().find(|&&(nx, ny)| self.cell(nx, ny).mine) {
            self.lose(mx, my);
            return;
        }
        for (nx, ny) in targets {
            self.flood(nx, ny);
        }
        self.check_win();
    }

    /// Right click: cycle blank -> flag -> question mark (if enabled) -> blank.
    pub fn toggle_mark(&mut self, x: usize, y: usize, question_marks: bool) {
        if self.is_over() {
            return;
        }
        let i = self.idx(x, y);
        let cell = &mut self.cells[i];
        if cell.revealed {
            return;
        }
        let next = match cell.mark {
            Mark::None => Mark::Flag,
            Mark::Flag if question_marks => Mark::Question,
            Mark::Flag | Mark::Question => Mark::None,
        };
        if cell.mark == Mark::Flag {
            self.flags -= 1;
        }
        if next == Mark::Flag {
            self.flags += 1;
        }
        cell.mark = next;
    }

    fn flood(&mut self, x: usize, y: usize) {
        let (w, h) = (self.config.width, self.config.height);
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            let i = self.idx(cx, cy);
            let cell = &mut self.cells[i];
            if cell.revealed || cell.mine || cell.mark == Mark::Flag {
                continue;
            }
            cell.revealed = true;
            cell.mark = Mark::None;
            self.revealed += 1;
            if cell.adjacent == 0 {
                stack.extend(neighbors(w, h, cx, cy));
            }
        }
    }

    fn lose(&mut self, x: usize, y: usize) {
        self.status = Status::Lost;
        self.exploded = Some((x, y));
    }

    fn check_win(&mut self) {
        let safe = self.cells.len() - self.config.mines;
        if self.revealed == safe {
            self.status = Status::Won;
            for cell in self.cells.iter_mut().filter(|c| c.mine) {
                cell.mark = Mark::Flag;
            }
            self.flags = self.config.mines;
        }
    }

    /// Places mines avoiding the first click and, when there is room, its neighbours
    /// too, so the opening move always clears an area.
    fn place_mines(&mut self, sx: usize, sy: usize) {
        let (w, h) = (self.config.width, self.config.height);
        let total = w * h;
        let mut safe = vec![false; total];
        safe[sy * w + sx] = true;
        let zone: Vec<_> = neighbors(w, h, sx, sy).collect();
        if total - 1 - zone.len() >= self.config.mines {
            for (x, y) in zone {
                safe[y * w + x] = true;
            }
        }
        let mut candidates: Vec<usize> = (0..total).filter(|&i| !safe[i]).collect();
        let count = self.config.mines.min(candidates.len());
        for k in 0..count {
            let j = k + self.rng.below(candidates.len() - k);
            candidates.swap(k, j);
            self.cells[candidates[k]].mine = true;
        }
        self.compute_adjacent();
    }

    fn compute_adjacent(&mut self) {
        let (w, h) = (self.config.width, self.config.height);
        for y in 0..h {
            for x in 0..w {
                let n = neighbors(w, h, x, y).filter(|&(nx, ny)| self.cells[ny * w + nx].mine).count();
                self.cells[y * w + x].adjacent = n as u8;
            }
        }
    }

    /// Test helper: a board in progress with mines at fixed positions.
    #[cfg(test)]
    fn with_mines(width: usize, height: usize, mines: &[(usize, usize)]) -> Game {
        let mut g = Game::new(Config { width, height, mines: mines.len() }, 0);
        for &(x, y) in mines {
            g.cells[y * width + x].mine = true;
        }
        g.compute_adjacent();
        g.status = Status::Playing;
        g
    }
}

/// Small xorshift64* PRNG so the game needs no extra dependencies and tests are deterministic.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // splitmix64 scramble so nearby seeds give unrelated boards.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 0x9E37_79B9_7F4A_7C15 } else { z })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mine_count(g: &Game) -> usize {
        g.cells.iter().filter(|c| c.mine).count()
    }

    #[test]
    fn first_click_is_safe_and_opens_an_area() {
        for seed in 0..200 {
            let mut g = Game::new(Config::EXPERT, seed);
            g.reveal(0, 0);
            assert_ne!(g.status(), Status::Lost, "seed {seed}");
            assert_eq!(g.cell(0, 0).adjacent, 0, "seed {seed}");
            assert_eq!(mine_count(&g), 99);
        }
    }

    #[test]
    fn first_click_is_safe_on_a_dense_board() {
        for seed in 0..50 {
            let cfg = Config::custom(9, 9, 64);
            let mut g = Game::new(cfg, seed);
            g.reveal(4, 4);
            assert_ne!(g.status(), Status::Lost);
            assert_eq!(mine_count(&g), 64);
        }
    }

    #[test]
    fn flood_fill_clears_board_and_wins() {
        let mut g = Game::with_mines(5, 5, &[(4, 4)]);
        g.reveal(0, 0);
        assert_eq!(g.status(), Status::Won);
        assert_eq!(g.cell(4, 4).mark, Mark::Flag);
        assert_eq!(g.mines_left(), 0);
    }

    #[test]
    fn revealing_a_mine_loses() {
        let mut g = Game::with_mines(3, 3, &[(1, 1)]);
        g.reveal(1, 1);
        assert_eq!(g.status(), Status::Lost);
        assert_eq!(g.exploded(), Some((1, 1)));
        g.reveal(0, 0);
        assert!(!g.cell(0, 0).revealed, "no moves after game over");
    }

    #[test]
    fn chord_with_correct_flags_opens_neighbours() {
        let mut g = Game::with_mines(3, 3, &[(0, 0)]);
        g.reveal(1, 1);
        assert_eq!(g.cell(1, 1).adjacent, 1);
        g.toggle_mark(0, 0, true);
        g.chord(1, 1);
        assert_eq!(g.status(), Status::Won);
    }

    #[test]
    fn chord_with_wrong_flag_loses() {
        let mut g = Game::with_mines(3, 3, &[(0, 0)]);
        g.reveal(1, 1);
        g.toggle_mark(2, 2, true);
        g.chord(1, 1);
        assert_eq!(g.status(), Status::Lost);
        assert_eq!(g.exploded(), Some((0, 0)));
    }

    #[test]
    fn chord_without_enough_flags_does_nothing() {
        let mut g = Game::with_mines(3, 3, &[(0, 0), (2, 2)]);
        g.reveal(1, 1);
        g.toggle_mark(0, 0, true);
        g.chord(1, 1);
        assert_eq!(g.status(), Status::Playing);
        assert!(!g.cell(2, 0).revealed);
    }

    #[test]
    fn marks_cycle_and_update_counter() {
        let mut g = Game::new(Config::BEGINNER, 1);
        g.toggle_mark(0, 0, true);
        assert_eq!((g.cell(0, 0).mark, g.mines_left()), (Mark::Flag, 9));
        g.toggle_mark(0, 0, true);
        assert_eq!((g.cell(0, 0).mark, g.mines_left()), (Mark::Question, 10));
        g.toggle_mark(0, 0, true);
        assert_eq!(g.cell(0, 0).mark, Mark::None);

        g.toggle_mark(0, 0, false);
        g.toggle_mark(0, 0, false);
        assert_eq!((g.cell(0, 0).mark, g.mines_left()), (Mark::None, 10));
    }

    #[test]
    fn flagged_cells_cannot_be_revealed() {
        let mut g = Game::new(Config::BEGINNER, 1);
        g.toggle_mark(3, 3, true);
        g.reveal(3, 3);
        assert_eq!(g.status(), Status::Ready);
        assert!(!g.cell(3, 3).revealed);
    }

    #[test]
    fn custom_config_is_clamped() {
        assert_eq!(Config::custom(1, 100, 5), Config { width: 9, height: 24, mines: 10 });
        assert_eq!(Config::custom(9, 9, 999).mines, 64);
    }
}
