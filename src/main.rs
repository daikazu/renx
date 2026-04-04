use std::io;
use std::path::Path;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};

use renx::app::App;
use renx::files;
use renx::input;
use renx::ui;

/// A TUI tool for bulk file renaming
#[derive(Parser, Debug)]
#[command(name = "renx", version, about)]
struct Args {
    /// Directory, pattern, or files (e.g. ".", "~/downloads", "~/downloads/*test*")
    #[arg(default_value = ".")]
    patterns: Vec<String>,

    /// Recurse into subdirectories
    #[arg(short, long)]
    recursive: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let (dir, filter) = resolve_args(&args);
    let file_list = load_files(&dir, args.recursive)?;

    if file_list.is_empty() {
        eprintln!("No files found in {}.", dir);
        return Ok(());
    }

    let mut app = App::new(file_list);

    if let Some(f) = filter {
        app.filter_text = f;
        app.update_filter();
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    if app.executed {
        println!("{}", app.status_message);
    }

    result
}

/// Expand ~ to $HOME
fn expand_tilde(s: &str) -> String {
    if s.starts_with("~/") || s == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}{}", home.to_string_lossy(), &s[1..]);
        }
    }
    s.to_string()
}

/// Figure out the directory and optional filter from CLI args.
///
/// Three cases:
///   1. Single arg, is a directory         -> (dir, None)
///   2. Single arg, contains wildcards     -> (parent_dir, Some(filter))
///   3. Multiple args (shell-expanded)     -> (common_parent, None)
fn resolve_args(args: &Args) -> (String, Option<String>) {
    // Single argument
    if args.patterns.len() == 1 {
        let expanded = expand_tilde(&args.patterns[0]);
        let path = Path::new(&expanded);

        // Directory
        if path.is_dir() {
            return (expanded.trim_end_matches('/').to_string(), None);
        }

        // Quoted glob pattern (wildcards survived the shell)
        if expanded.contains('*') || expanded.contains('?') {
            let first_wild = expanded
                .find(|c: char| c == '*' || c == '?')
                .unwrap_or(expanded.len());
            let dir_end = expanded[..first_wild].rfind('/');

            return match dir_end {
                Some(pos) => {
                    let dir = &expanded[..pos];
                    let filter = &expanded[pos + 1..];
                    (
                        if dir.is_empty() { ".".to_string() } else { dir.to_string() },
                        Some(filter.to_string()),
                    )
                }
                None => (".".to_string(), Some(expanded)),
            };
        }

        // Treat as directory path even if it doesn't exist yet
        return (expanded, None);
    }

    // Multiple arguments — shell expanded the glob.
    // Extract parent dir and try to reconstruct the filter pattern.
    let expanded: Vec<String> = args.patterns.iter().map(|p| expand_tilde(p)).collect();
    let first_path = Path::new(&expanded[0]);
    let dir = first_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let dir = if dir.is_empty() { ".".to_string() } else { dir };

    // Try to infer filter: if all files share a common extension, use *.ext
    // If all filenames contain a common substring, use *substring*
    let filenames: Vec<String> = expanded
        .iter()
        .filter_map(|p| Path::new(p).file_name().map(|f| f.to_string_lossy().to_string()))
        .collect();

    let filter = infer_filter(&filenames);

    (dir, filter)
}

/// Try to reconstruct a wildcard filter from shell-expanded filenames.
/// e.g. all ".jpeg" files -> "*.jpeg", all containing "test" -> "*test*"
fn infer_filter(filenames: &[String]) -> Option<String> {
    if filenames.is_empty() {
        return None;
    }

    // Check if all share a common extension
    let first_ext = Path::new(&filenames[0])
        .extension()
        .map(|e| e.to_string_lossy().to_string());

    if let Some(ref ext) = first_ext {
        if filenames.iter().all(|f| {
            Path::new(f)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .as_ref()
                == Some(ext)
        }) {
            return Some(format!("*.{}", ext));
        }
    }

    // Check if all share a common substring (at least 3 chars) in the stem
    let stems: Vec<String> = filenames
        .iter()
        .filter_map(|f| {
            Path::new(f)
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
        })
        .collect();

    if stems.len() >= 2 {
        let first = &stems[0];
        let chars: Vec<char> = first.chars().collect();
        for len in (3..=chars.len()).rev() {
            for start in 0..=chars.len() - len {
                let candidate: String = chars[start..start + len].iter().collect();
                if stems.iter().all(|s| s.contains(&candidate)) {
                    return Some(format!("*{}*", candidate));
                }
            }
        }
    }

    None
}

/// Load all files from a directory.
fn load_files(dir: &str, recursive: bool) -> io::Result<Vec<files::FileEntry>> {
    let path = Path::new(dir);
    let mut all_files = if recursive {
        let glob_pattern = format!("{}/**/*", dir.trim_end_matches('/'));
        files::discover_glob(&glob_pattern)?
    } else {
        files::discover_dir(path)?
    };

    all_files.sort_by(|a, b| a.full_path().cmp(&b.full_path()));
    all_files.dedup_by(|a, b| a.full_path() == b.full_path());

    Ok(all_files)
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                input::handle_key(app, key);
                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}
