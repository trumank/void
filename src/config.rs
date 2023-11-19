use std::{
    collections::HashMap,
    env, fmt,
    fs::File,
    io::{self, Error, ErrorKind, Read},
};

use log::{error, warn};
use termion::event::{Event, Key, MouseEvent};

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Action {
    Mode(Mode),
    LeftClick(u16, u16),
    RightClick(u16, u16),
    Release(u16, u16),
    Char(Mode, char),
    UnselectRet,
    ScrollUp,
    ScrollDown,
    DeleteSelected,
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,
    EraseChar,
    CreateSibling,
    CreateChild,
    CreateFreeNode,
    ExecSelected,
    DrillDown,
    PopUp,
    PrefixJump,
    ToggleCompleted,
    ToggleHideCompleted,
    Arrow,
    AutoArrange,
    ToggleCollapsed,
    Quit,
    Save,
    ToggleShowLogs,
    EnterCmd,
    FindTask,
    YankPasteNode,
    RaiseSelected,
    LowerSelected,
    Search,
    UndoDelete,
    Help,
    SelectParent,
    SelectNextSibling,
    SelectPrevSibling,
}

fn to_action(input: String) -> Option<Action> {
    match &*input {
        "unselect" => Some(Action::UnselectRet),
        "scroll_up" => Some(Action::ScrollUp),
        "scroll_down" => Some(Action::ScrollDown),
        "delete" => Some(Action::DeleteSelected),
        "select_up" => Some(Action::SelectUp),
        "select_down" => Some(Action::SelectDown),
        "select_left" => Some(Action::SelectLeft),
        "select_right" => Some(Action::SelectRight),
        "erase" => Some(Action::EraseChar),
        "create_sibling" => Some(Action::CreateSibling),
        "create_child" => Some(Action::CreateChild),
        "create_free_node" => Some(Action::CreateFreeNode),
        "execute" => Some(Action::ExecSelected),
        "drill_down" => Some(Action::DrillDown),
        "pop_up" => Some(Action::PopUp),
        "jump" => Some(Action::PrefixJump),
        "toggle_completed" => Some(Action::ToggleCompleted),
        "toggle_hide_completed" => Some(Action::ToggleHideCompleted),
        "arrow" => Some(Action::Arrow),
        "auto_arrange" => Some(Action::AutoArrange),
        "toggle_collapsed" => Some(Action::ToggleCollapsed),
        "quit" => Some(Action::Quit),
        "save" => Some(Action::Save),
        "toggle_show_logs" => Some(Action::ToggleShowLogs),
        "enter_command" => Some(Action::EnterCmd),
        "find_task" => Some(Action::FindTask),
        "yank_paste_node" => Some(Action::YankPasteNode),
        "raise_selected" => Some(Action::RaiseSelected),
        "lower_selected" => Some(Action::LowerSelected),
        "search" => Some(Action::Search),
        "undo_delete" => Some(Action::UndoDelete),
        "help" => Some(Action::Help),
        "select_parent" => Some(Action::SelectParent),
        "select_next_sibling" => Some(Action::SelectNextSibling),
        "select_prev_sibling" => Some(Action::SelectPrevSibling),
        _ => None,
    }
}

// Alt and Control must be specified with capital letters C- and A-
fn to_key(raw_key: String) -> Option<Key> {
    use termion::event::Key::{Alt, Char, Ctrl};

    fn extract_key(raw_key: &str, idx: usize) -> Option<char> { raw_key.chars().nth(idx) }

    match &*raw_key {
        "esc" => Some(Key::Esc),
        "pgup" => Some(Key::PageUp),
        "pgdn" => Some(Key::PageDown),
        "del" => Some(Key::Delete),
        "backspace" => Some(Key::Backspace),
        "up" => Some(Key::Up),
        "down" => Some(Key::Down),
        "left" => Some(Key::Left),
        "right" => Some(Key::Right),

        "space" => Some(Char(' ')),
        "enter" => Some(Char('\n')),
        "tab" => Some(Char('\t')),

        key if key.len() == 1 => extract_key(key, 0).map(Char),

        key if key.starts_with("A-") => extract_key(key, 2).map(Alt),
        key if key.starts_with("C-") => extract_key(key, 2).map(Ctrl),

        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    config: HashMap<(Mode, Key), Action>,
}

impl Default for Config {
    fn default() -> Config {
        use termion::event::Key::*;
        Config {
            config: [
                ((Mode::Normal, Char('i')), Action::Mode(Mode::Insert)),
                ((Mode::Normal, Char('A')), Action::Mode(Mode::Insert)),
                ((Mode::Insert, Esc), Action::Mode(Mode::Normal)),
                ((Mode::Normal, PageUp), Action::ScrollUp),
                ((Mode::Normal, PageDown), Action::ScrollDown),
                ((Mode::Normal, Delete), Action::DeleteSelected),
                ((Mode::Normal, Char('k')), Action::SelectUp),
                ((Mode::Normal, Char('j')), Action::SelectDown),
                ((Mode::Normal, Char('h')), Action::SelectLeft),
                ((Mode::Normal, Char('l')), Action::SelectRight),
                ((Mode::Insert, Backspace), Action::EraseChar),
                ((Mode::Normal, F(1)), Action::PrefixJump),
                ((Mode::Normal, Char('o')), Action::CreateSibling),
                ((Mode::Normal, Char('\t')), Action::CreateChild),
                ((Mode::Normal, Char('n')), Action::CreateFreeNode),
                ((Mode::Normal, Ctrl('k')), Action::ExecSelected),
                ((Mode::Normal, Ctrl('w')), Action::DrillDown),
                ((Mode::Normal, Ctrl('q')), Action::PopUp),
                ((Mode::Normal, Char('f')), Action::PrefixJump),
                ((Mode::Normal, Ctrl('a')), Action::ToggleCompleted),
                ((Mode::Normal, Ctrl('h')), Action::ToggleHideCompleted),
                ((Mode::Normal, Ctrl('r')), Action::Arrow),
                ((Mode::Normal, Ctrl('p')), Action::AutoArrange),
                ((Mode::Normal, Char(' ')), Action::ToggleCollapsed),
                ((Mode::Normal, Ctrl('c')), Action::Quit),
                ((Mode::Normal, Ctrl('x')), Action::Save),
                ((Mode::Normal, Ctrl('l')), Action::ToggleShowLogs),
                ((Mode::Normal, Char(':')), Action::EnterCmd),
                ((Mode::Normal, Ctrl('v')), Action::FindTask),
                ((Mode::Normal, Char('y')), Action::YankPasteNode),
                ((Mode::Normal, Char('K')), Action::RaiseSelected),
                ((Mode::Normal, Char('J')), Action::LowerSelected),
                ((Mode::Normal, Char('/')), Action::Search),
                ((Mode::Normal, Char('u')), Action::UndoDelete),
                ((Mode::Normal, Ctrl('?')), Action::Help),
                ((Mode::Normal, Alt('P')), Action::SelectParent),
                ((Mode::Normal, Ctrl('n')), Action::SelectNextSibling),
                ((Mode::Normal, Ctrl('p')), Action::SelectPrevSibling),
            ]
            .into(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Configured Hotkeys:").unwrap();
        for (key, action) in &self.config {
            writeln!(f, "    {:?}: {:?}", action, key).unwrap();
        }
        Ok(())
    }
}

impl Config {
    pub fn maybe_parsed_from_env() -> io::Result<Config> {
        if let Ok(p) = env::var("KEYFILE") {
            Config::parse_keyfile(p)
        } else {
            Ok(Config::default())
        }
    }

    pub fn parse_keyfile(p: String) -> io::Result<Config> {
        let mut buf = String::new();
        let mut f = File::open(p)?;
        f.read_to_string(&mut buf)?;
        let config = Config::default();
        for (mut line_num, line) in buf.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Zero based indexing inappropriate here.
            line_num += 1;

            let parts: Vec<_> = line.splitn(2, ':').map(|p| p.trim()).collect();
            if parts.len() != 2 {
                let e = format!("No colon found on line {}", line_num);
                error!("{}", e);
                return Err(Error::new(ErrorKind::Other, e));
            }

            let (raw_action, raw_key) = (parts[0], parts[1]);

            let key_opt = to_key(raw_key.to_owned());
            let action_opt = to_action(raw_action.to_owned());

            if key_opt.is_none() || action_opt.is_none() {
                let e = format!("invalid config at line {}: {}", line_num, line);
                error!("{}", e);
                return Err(Error::new(ErrorKind::Other, e));
            }

            let key = key_opt.unwrap();
            let action = action_opt.unwrap();

            todo!()
            // config.config.insert(key, action);
        }

        Ok(config)
    }

    pub fn map(&self, e: Event, mode: Mode) -> Option<Action> {
        use termion::event::{Key::*, MouseButton};
        match e {
            Event::Key(Char(c)) => {
                if let Some(action) = self.config.get(&(mode, Char(c))).cloned() {
                    Some(action)
                } else {
                    Some(Action::Char(mode, c))
                }
            },
            Event::Mouse(MouseEvent::Press(MouseButton::Right, x, y)) => {
                Some(Action::RightClick(x, y))
            },
            Event::Mouse(MouseEvent::Press(_, x, y)) => Some(Action::LeftClick(x, y)),
            Event::Mouse(MouseEvent::Release(x, y)) => Some(Action::Release(x, y)),
            Event::Mouse(MouseEvent::Hold(..)) => None,
            Event::Key(other) => {
                let lookup = self.config.get(&(mode, other)).cloned();
                if lookup.is_none() {
                    warn!("Weird event {:?}", other);
                }
                lookup
            },
            other => {
                warn!("Unknown event received: {:?}", other);
                None
            },
        }
    }
}
