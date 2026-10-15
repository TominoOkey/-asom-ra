use super::*;

#[cfg(unix)]
pub(super) fn launch_with_pty(
    app: &PathBuf,
    socket_override: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::fd::{FromRawFd, RawFd};
    let app = if app.is_absolute() {
        app.clone()
    } else {
        std::env::current_dir()?.join(app)
    };
    if !app.exists() {
        return Err(format!("Aplikace nebyla nalezena: {}", app.display()).into());
    }
    let mut masters: Vec<RawFd> = Vec::new();
    let mut slaves = Vec::new();
    let mut keepalive_slaves: Vec<RawFd> = Vec::new();
    for _ in 0..3 {
        let mut master: libc::c_int = -1;
        let mut slave: libc::c_int = -1;
        if unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let mut name = vec![0i8; 256];
        if unsafe { libc::ttyname_r(slave, name.as_mut_ptr(), name.len()) } != 0 {
            unsafe {
                libc::close(master);
                libc::close(slave);
            }
            return Err(io::Error::last_os_error().into());
        }
        let path = unsafe { std::ffi::CStr::from_ptr(name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(slave, termios.as_mut_ptr()) } == 0 {
            let mut termios = unsafe { termios.assume_init() };
            unsafe {
                libc::cfmakeraw(&mut termios);
                libc::tcsetattr(slave, libc::TCSANOW, &termios);
            }
        }
        let flags = unsafe { libc::fcntl(master, libc::F_GETFL) };
        unsafe {
            libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        masters.push(master);
        // Keep one slave descriptor open until shutdown. This prevents EIO on
        // the master while the GUI is starting or before the user connects.
        keepalive_slaves.push(slave);
        slaves.push(path);
    }
    let mut child: Child = Command::new(&app)
        .current_dir(app.parent().unwrap_or(std::path::Path::new(".")))
        .env("CASOMIRA_SIM_INPUT_PORT", &slaves[0])
        .env(
            "CASOMIRA_SIM_OUTPUT_PORTS",
            format!("{}|{}", slaves[1], slaves[2]),
        )
        .stdin(Stdio::null())
        .spawn()?;
    println!(
        "Pseudoporty připravené. Spustil jsem aplikaci {}.",
        app.display()
    );
    println!("Příkazy: start | mezičas DRÁHA ČAS_MS | maska MASKA ČAS_HODNOTA | konec");

    let socket_path = socket_override
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("COM20"));
    let parallels = if socket_path.exists() {
        match std::os::unix::net::UnixStream::connect(&socket_path) {
            Ok(stream) => {
                println!(
                    "Výstup COM19 se přeposílá do Parallels COM1 přes {}.",
                    socket_path.display()
                );
                Some(stream)
            }
            Err(error) => {
                eprintln!(
                    "Nelze se připojit k Parallels socketu {}: {error}",
                    socket_path.display()
                );
                None
            }
        }
    } else {
        println!(
            "Parallels socket pro COM1 nebyl nalezen: {}",
            socket_path.display()
        );
        None
    };

    let out_fd_1 = unsafe { libc::dup(masters[1]) };
    let out_fd_2 = unsafe { libc::dup(masters[2]) };
    for (fd, name, forward) in [
        (out_fd_1, "výstup 1", parallels),
        (out_fd_2, "výstup 2", None),
    ] {
        thread::spawn(move || {
            if fd < 0 {
                return;
            }
            let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
            let mut buffer = Vec::new();
            let mut bytes = [0u8; 128];
            let mut forward = forward;
            loop {
                match std::io::Read::read(&mut file, &mut bytes) {
                    Ok(0) => thread::sleep(Duration::from_millis(20)),
                    Ok(n) => {
                        if let Some(stream) = forward.as_mut() {
                            if let Err(error) = stream.write_all(&bytes[..n]) {
                                eprintln!("Přenos do Parallels COM1 selhal: {error}");
                                forward = None;
                            }
                        }
                        buffer.extend_from_slice(&bytes[..n]);
                        while buffer.len() >= 15 {
                            let rec: Vec<_> = buffer.drain(..15).collect();
                            println!("PŘIJATO {name}: {}", String::from_utf8_lossy(&rec));
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
        });
    }
    let stdin = io::stdin();
    loop {
        print!("sim> ");
        io::stdout().flush()?;
        let mut command = String::new();
        if stdin.lock().read_line(&mut command)? == 0 {
            break;
        }
        let fields: Vec<_> = command.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let payload = match fields[0].to_lowercase().as_str() {
            "konec" | "quit" | "exit" => break,
            "start" if fields.len() == 1 => Some("255;255\r\n".to_string()),
            "mezičas" | "split" if fields.len() == 3 => {
                let lane: u8 = fields[1].parse()?;
                let ms: u32 = fields[2].parse()?;
                if !(1..=8).contains(&lane) {
                    eprintln!("Zadej dráhu 1 až 8.");
                    continue;
                }
                let mut raw = (ms / 10).min(254) as u16;
                if raw == 255 {
                    raw = 256;
                }
                Some(format!("{};{raw}\r\n", 1u16 << (lane - 1)))
            }
            "maska" if fields.len() == 3 => {
                let mask: u16 = fields[1].parse()?;
                let time: u16 = fields[2].parse()?;
                if mask > 255 || time > 255 {
                    eprintln!("Maska a čas musejí být 0 až 255.");
                    continue;
                }
                Some(format!("{mask};{time}\r\n"))
            }
            "help" | "nápověda" => {
                println!("start | mezičas DRÁHA ČAS_MS | maska MASKA ČAS_HODNOTA | konec");
                None
            }
            _ => {
                eprintln!("Neznámý příkaz. Zadej help.");
                None
            }
        };
        if let Some(line) = payload {
            let bytes = line.as_bytes();
            let result = unsafe { libc::write(masters[0], bytes.as_ptr().cast(), bytes.len()) };
            if result < 0 {
                eprintln!(
                    "Nepodařilo se odeslat událost: {}",
                    io::Error::last_os_error()
                );
            } else {
                println!("ODESLÁNO časomíře: {}", line.trim());
            }
        }
        if child.try_wait()?.is_some() {
            eprintln!("Aplikace časomíry byla ukončena.");
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    for fd in masters {
        unsafe {
            libc::close(fd);
        }
    }
    for fd in keepalive_slaves {
        unsafe {
            libc::close(fd);
        }
    }
    Ok(())
}
