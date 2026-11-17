//! Winning times for the preset difficulties, kept in a small text file so they last
//! between sessions. Each line is `mode<TAB>milliseconds<TAB>unix-seconds`.

use crate::game::Mode;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

pub const RANKED: [Mode; 3] = [Mode::Beginner, Mode::Intermediate, Mode::Expert];
pub const TOP: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub mode: Mode,
    pub millis: u64,
    /// When the run finished, in seconds since the Unix epoch.
    pub when: u64,
}

/// Where a run landed among every win recorded for its difficulty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// 1-based; ties share the better place.
    pub rank: usize,
    pub total: usize,
    /// Share of the other runs this one beat, 0-100. A first run counts as 100.
    pub percentile: u32,
}

pub fn mode_key(mode: Mode) -> Option<&'static str> {
    match mode {
        Mode::Beginner => Some("beginner"),
        Mode::Intermediate => Some("intermediate"),
        Mode::Expert => Some("expert"),
        Mode::Custom => None,
    }
}

fn mode_from_key(key: &str) -> Option<Mode> {
    RANKED.into_iter().find(|&m| mode_key(m) == Some(key))
}

fn parse(contents: &str) -> Vec<Run> {
    contents
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let mode = mode_from_key(fields.next()?)?;
            let millis = fields.next()?.parse().ok()?;
            let when = fields.next()?.parse().ok()?;
            Some(Run { mode, millis, when })
        })
        .collect()
}

fn line(run: &Run) -> String {
    format!("{}\t{}\t{}\n", mode_key(run.mode).unwrap_or("custom"), run.millis, run.when)
}

/// `$XDG_DATA_HOME/minesweeper` (usually `~/.local/share/minesweeper`) on Linux, and
/// `~/Library/Application Support/Minesweeper` on macOS.
fn default_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        home?.join("Library/Application Support/Minesweeper")
    } else {
        match std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
            Some(d) => PathBuf::from(d).join("minesweeper"),
            None => home?.join(".local/share/minesweeper"),
        }
    };
    Some(dir.join("leaderboard.tsv"))
}

pub struct Leaderboard {
    path: Option<PathBuf>,
    runs: Vec<Run>,
}

impl Leaderboard {
    pub fn load() -> Self {
        Self::load_from(default_path())
    }

    /// A missing file is an empty leaderboard; unreadable lines are skipped.
    pub fn load_from(path: Option<PathBuf>) -> Self {
        let runs = path.as_ref().and_then(|p| fs::read_to_string(p).ok()).map(|s| parse(&s)).unwrap_or_default();
        Leaderboard { path, runs }
    }

    /// Adds a win and appends it to the file. The run counts for this session even if
    /// saving fails; the error is returned so the player can be told.
    pub fn record(&mut self, run: Run) -> (Placement, io::Result<()>) {
        self.runs.push(run);
        let placement = self.placement(&run);
        (placement, self.append(&run))
    }

    fn append(&self, run: &Run) -> io::Result<()> {
        let path = self.path.as_ref().ok_or_else(|| io::Error::other("no home directory"))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        OpenOptions::new().create(true).append(true).open(path)?.write_all(line(run).as_bytes())
    }

    /// Every win for `mode`, fastest first; earlier runs win ties.
    pub fn ranked(&self, mode: Mode) -> Vec<Run> {
        let mut runs: Vec<Run> = self.runs.iter().copied().filter(|r| r.mode == mode).collect();
        runs.sort_by_key(|r| (r.millis, r.when));
        runs
    }

    pub fn top(&self, mode: Mode) -> Vec<Run> {
        let mut runs = self.ranked(mode);
        runs.truncate(TOP);
        runs
    }

    pub fn placement(&self, run: &Run) -> Placement {
        let times: Vec<u64> = self.runs.iter().filter(|r| r.mode == run.mode).map(|r| r.millis).collect();
        let faster = times.iter().filter(|&&t| t < run.millis).count();
        let slower = times.iter().filter(|&&t| t > run.millis).count();
        let others = times.len().saturating_sub(1);
        let percentile = if others == 0 { 100 } else { (100 * slower / others) as u32 };
        Placement { rank: faster + 1, total: times.len(), percentile }
    }
}

/// "1st", "2nd", "3rd", "11th", "22nd"...
pub fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

pub fn format_time(millis: u64) -> String {
    format!("{}.{:02}", millis / 1000, millis % 1000 / 10)
}

/// The local calendar date of a Unix timestamp, as YYYY-MM-DD.
pub fn format_date(when: u64) -> String {
    #[cfg(unix)]
    {
        let t = when as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        if !unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
            return format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
        }
    }
    // UTC fallback (Howard Hinnant's days-to-civil).
    let z = (when / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(mode: Mode, millis: u64, when: u64) -> Run {
        Run { mode, millis, when }
    }

    fn board(runs: &[Run]) -> Leaderboard {
        Leaderboard { path: None, runs: runs.to_vec() }
    }

    #[test]
    fn parse_round_trips_and_skips_junk() {
        let runs = [run(Mode::Beginner, 12_345, 1), run(Mode::Expert, 99_000, 2)];
        let mut text: String = runs.iter().map(line).collect();
        text.push_str("garbage\nbeginner\tnope\t3\ncustom\t1\t2\n");
        assert_eq!(parse(&text), runs);
    }

    #[test]
    fn top_is_fastest_ten_for_that_mode_only() {
        let mut runs: Vec<Run> = (0..15).map(|i| run(Mode::Beginner, 20_000 - i * 1000, i)).collect();
        runs.push(run(Mode::Expert, 1, 99));
        let top = board(&runs).top(Mode::Beginner);
        assert_eq!(top.len(), TOP);
        assert_eq!(top[0].millis, 6_000);
        assert!(top.windows(2).all(|w| w[0].millis <= w[1].millis));
        assert!(top.iter().all(|r| r.mode == Mode::Beginner));
    }

    #[test]
    fn placement_ranks_against_every_win_in_the_mode() {
        let runs: Vec<Run> = [10, 20, 30, 40, 50].iter().map(|&t| run(Mode::Intermediate, t, t)).collect();
        let b = board(&runs);
        assert_eq!(b.placement(&runs[0]), Placement { rank: 1, total: 5, percentile: 100 });
        assert_eq!(b.placement(&runs[2]), Placement { rank: 3, total: 5, percentile: 50 });
        assert_eq!(b.placement(&runs[4]), Placement { rank: 5, total: 5, percentile: 0 });
    }

    #[test]
    fn ties_share_the_better_place() {
        let runs = [run(Mode::Beginner, 10, 1), run(Mode::Beginner, 10, 2), run(Mode::Beginner, 20, 3)];
        let p = board(&runs).placement(&runs[1]);
        assert_eq!((p.rank, p.percentile), (1, 50));
    }

    #[test]
    fn first_run_is_first_and_hundredth_percentile() {
        let mut b = board(&[]);
        let (p, saved) = b.record(run(Mode::Expert, 5, 1));
        assert_eq!(p, Placement { rank: 1, total: 1, percentile: 100 });
        assert!(saved.is_err(), "no path, so nothing to save to");
    }

    #[test]
    fn record_appends_to_the_file() {
        let dir = std::env::temp_dir().join(format!("minesweeper-test-{}", std::process::id()));
        let path = dir.join("nested/leaderboard.tsv");
        let mut b = Leaderboard::load_from(Some(path.clone()));
        b.record(run(Mode::Beginner, 1500, 7)).1.unwrap();
        b.record(run(Mode::Beginner, 900, 8)).1.unwrap();
        let reloaded = Leaderboard::load_from(Some(path));
        assert_eq!(reloaded.ranked(Mode::Beginner), vec![run(Mode::Beginner, 900, 8), run(Mode::Beginner, 1500, 7)]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn formatting() {
        assert_eq!(
            [1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111].map(ordinal),
            ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "101st", "111th"]
        );
        assert_eq!(format_time(12_345), "12.34");
        assert_eq!(format_time(999_999), "999.99");
        assert_eq!(format_date(0).len(), 10);
    }
}
