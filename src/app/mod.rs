use super::*;

mod imports;
mod io;
mod overlay_data;
mod race;
mod selection;
mod state;

use self::race::remembered_race_folder;

pub(super) struct App {
    race_folder: Option<PathBuf>,
    race_selected: bool,
    timing_log: VecDeque<TimingHeat>,
    input_name: String,
    ports: Vec<String>,
    input: Option<SerialLink>,
    outputs: Vec<(String, SerialLink)>,
    rx: Option<Receiver<Result<String, String>>>,
    connected: bool,
    status: String,
    discipline: u16,
    heat_no: u8,
    splits_target: u8,
    heat: Heat,
    heat_activity: bool,
    started_at: Option<Instant>,
    race_elapsed_ms: Option<u32>,
    selected_delayed: Option<Instant>,
    selection_reset_pending: bool,
    orders: BTreeMap<u8, u8>,
    latest: BTreeMap<u8, u32>,
    previous: BTreeMap<u8, u32>,
    finished: BTreeSet<u8>,
    startlist: StartList,
    events: Vec<(u16, u8, String)>,
    event_index: usize,
    discipline_map: BTreeMap<u16, (u16, String)>,
    log_dir: PathBuf,
    event_name: String,
    log_error: Option<String>,
    timer_armed: bool,
    show_live: bool,
    obs_logos: Vec<String>,
    ad_speed_percent: u8,
    ad_logos_white: bool,
    results_mode: bool,
    results_due: Option<Instant>,
    results_auto_next: bool,
    live_presentation: bool,
    results_snapshot: Option<OverlayData>,
    simulator_visible: bool,
    simulator_started: Option<Instant>,
    simulator_last: String,
    pending_confirm: Option<(u16, u8, String)>,
    reset_confirm_pending: bool,
    selection_flash: Option<(String, bool, Instant)>,
    show_settings: bool,
    shortcuts: ShortcutSettings,
    shortcut_capture: Option<ShortcutAction>,
    hotkey_manager: Option<GlobalHotKeyManager>,
    hotkey_actions: BTreeMap<u32, ShortcutAction>,
    hotkey_error: Option<String>,
    overlay_state: Arc<RwLock<OverlayState>>,
}

#[derive(Deserialize, Serialize)]
struct TimingHeat {
    discipline: u16,
    heat: u8,
    title: String,
    started: bool,
    splits: BTreeMap<u8, BTreeMap<u8, u32>>,
}

impl App {
    pub(super) fn new() -> Self {
        let overlay_state = Arc::new(RwLock::new(empty_overlay_state()));
        start_overlay_server(Arc::clone(&overlay_state));
        let mut obs_logos: Vec<String> = std::fs::read_dir(crate::overlay::logo_directory())
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        obs_logos.sort();
        let mut app = Self {
            race_folder: None,
            race_selected: false,
            timing_log: VecDeque::new(),
            input_name: String::new(),
            ports: Vec::new(),
            input: None,
            outputs: Vec::new(),
            rx: None,
            connected: false,
            status: "Vyberte existující závod nebo vytvořte nový".into(),
            discipline: 1,
            heat_no: 1,
            splits_target: 1,
            heat: Heat::new(1, 1, 1),
            heat_activity: false,
            started_at: None,
            race_elapsed_ms: None,
            selected_delayed: None,
            selection_reset_pending: false,
            orders: BTreeMap::new(),
            latest: BTreeMap::new(),
            previous: BTreeMap::new(),
            finished: BTreeSet::new(),
            startlist: StartList::new(),
            events: Vec::new(),
            event_index: 0,
            log_dir: PathBuf::new(),
            event_name: String::new(),
            log_error: None,
            discipline_map: BTreeMap::new(),
            timer_armed: false,
            show_live: true,
            obs_logos,
            ad_speed_percent: load_ad_speed_percent(),
            ad_logos_white: load_ad_logos_white(),
            results_mode: false,
            results_due: None,
            results_auto_next: false,
            live_presentation: true,
            results_snapshot: None,
            simulator_visible: false,
            simulator_started: None,
            simulator_last: String::new(),
            pending_confirm: None,
            reset_confirm_pending: false,
            selection_flash: None,
            show_settings: false,
            shortcuts: load_shortcuts(),
            shortcut_capture: None,
            hotkey_manager: None,
            hotkey_actions: BTreeMap::new(),
            hotkey_error: None,
            overlay_state,
        };
        app.register_global_shortcuts();
        app.refresh_ports();
        #[cfg(target_os = "windows")]
        if let Err(error) = ensure_com0com_pair() {
            app.status = error;
        }
        if let Some(folder) = remembered_race_folder() {
            let _ = app.load_race_folder(&folder);
        }
        app
    }
}

fn ad_speed_settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".casomira_ad_speed"))
}

fn load_ad_speed_percent() -> u8 {
    ad_speed_settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| value.trim().parse::<u8>().ok())
        .filter(|value| *value <= 100)
        .unwrap_or(30)
}

fn save_ad_speed_percent(value: u8) {
    if let Some(path) = ad_speed_settings_path() {
        let _ = std::fs::write(path, value.to_string());
    }
}

fn ad_logos_white_settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".casomira_ad_logos_white"))
}

fn load_ad_logos_white() -> bool {
    ad_logos_white_settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .is_some_and(|value| value.trim() == "true")
}

fn save_ad_logos_white(value: bool) {
    if let Some(path) = ad_logos_white_settings_path() {
        let _ = std::fs::write(path, value.to_string());
    }
}

fn chrono_free_timestamp() -> String {
    // Local timestamp formatting without depending on a heavyweight date library.
    format!(
        "{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
    )
}

#[cfg(target_os = "windows")]
fn ensure_com0com_pair() -> Result<(), String> {
    use std::process::Command;

    let ports = available_ports().map_err(|error| format!("Nelze ověřit COM porty: {error}"))?;
    let has = |name: &str| {
        ports
            .iter()
            .any(|port| port.port_name.eq_ignore_ascii_case(name))
    };
    if has("COM19") && has("COM20") {
        return Ok(());
    }

    let mut candidates = vec![PathBuf::from("setupc.exe"), PathBuf::from("setupc")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("drivers/com0com/setupc.exe"));
            candidates.push(dir.join("com0com/setupc.exe"));
        }
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("com0com/setupc.exe"));
    }
    let setupc = candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| which_setupc());
    let Some(setupc) = setupc else {
        return Err("Chybí ovladač virtuálních portů com0com. Nainstalujte com0com a vytvořte jeden pár COM19 ↔ COM20, potom aplikaci spusťte znovu.".into());
    };

    let mut command = Command::new(&setupc);
    command
        .arg("install")
        .arg("PortName=COM19")
        .arg("PortName=COM20");
    if let Some(directory) = setupc.parent() {
        command.current_dir(directory);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = command
        .output()
        .map_err(|error| format!("Nelze spustit {}: {error}", setupc.display()))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Vytvoření páru COM19 ↔ COM20 se nezdařilo ({}). Spusťte aplikaci jako správce nebo nainstalujte com0com ručně. {detail}",
            output.status
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn which_setupc() -> Option<PathBuf> {
    use std::process::Command;
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for name in ["setupc.exe", "setupc"] {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let _ = Command::new("where.exe").arg("setupc.exe").output();
    None
}

pub(super) mod ui;
