use super::*;

impl App {
    pub(super) fn refresh_ports(&mut self) {
        self.ports = available_ports()
            .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
            .unwrap_or_default();
        if let Ok(input) = std::env::var("CASOMIRA_SIM_INPUT_PORT") {
            self.ports = vec![input];
        }
        self.ports
            .sort_by_key(|p| (!p.to_uppercase().starts_with("COM"), p.to_lowercase()));
        if !self.ports.contains(&self.input_name) {
            self.input_name = self
                .ports
                .iter()
                .find(|p| p.eq_ignore_ascii_case("COM3"))
                .or(self.ports.first())
                .cloned()
                .unwrap_or_default();
        }
    }

    pub(super) fn ensure_logs(&mut self) {
        for name in [
            "casomira.log",
            "casomira_hardware_vstup.log",
            "casomira_do_casomiry.log",
            "casomira_do_com.log",
        ] {
            let path = self.log_dir.join(name);
            if !path.exists() || path.metadata().map(|m| m.len() == 0).unwrap_or(true) {
                let header = if name == "casomira.log" {
                    "timestamp | IN | IN_TX | OUT | EVENT | details\n"
                } else {
                    "timestamp | DATA\n"
                };
                if let Ok(mut file) = File::create(path) {
                    let _ = file.write_all(header.as_bytes());
                }
            }
        }
    }

    pub(super) fn log(&mut self, category: &str, value: &str, details: &str) {
        let timestamp = chrono_free_timestamp();
        let escaped = |s: &str| {
            s.replace('\r', "\\r")
                .replace('\n', "\\n")
                .replace('|', "\\|")
        };
        let mut fields = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let idx = match category {
            "IN" => Some(0),
            "IN_TX" => Some(1),
            "OUT" => Some(2),
            "EVENT" => Some(3),
            _ => None,
        };
        if let Some(idx) = idx {
            fields[idx] = format!("{}={}", category, escaped(value));
        }
        if !details.is_empty() {
            fields[4] = format!("details={}", escaped(details));
        }
        let line = std::iter::once(timestamp.clone())
            .chain(fields.iter().cloned())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" | ")
            + "\n";
        let mut paths = vec![(self.log_dir.join("casomira.log"), line)];
        let secondary = match category {
            "IN" => Some("casomira_hardware_vstup.log"),
            "IN_TX" => Some("casomira_do_casomiry.log"),
            "OUT" => Some("casomira_do_com.log"),
            _ => None,
        };
        if let Some(name) = secondary {
            paths.push((
                self.log_dir.join(name),
                format!("{timestamp} | DATA={}\n", escaped(value)),
            ));
        }
        for (path, content) in paths {
            let result = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut f| f.write_all(content.as_bytes()));
            if let Err(err) = result {
                self.log_error = Some(err.to_string());
            }
        }
    }

    pub(super) fn connect(&mut self) {
        if self.input_name.is_empty() {
            self.status = "Vyberte vstupní sériový port".into();
            return;
        }
        if OUTPUT_PORTS
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&self.input_name))
        {
            self.status = "Vstup nesmí být COM19, protože je určený pro výstup".into();
            return;
        }
        let mut outputs = Vec::new();
        let sim_out = std::env::var("CASOMIRA_SIM_OUTPUT_PORTS").ok();
        let output_names: Vec<(String, bool)> = sim_out
            .map(|s| s.split('|').map(|s| (s.to_string(), true)).collect())
            .unwrap_or_else(|| {
                OUTPUT_PORTS
                    .iter()
                    .map(|p| (p.to_string(), false))
                    .collect()
            });
        for (name, pseudo) in &output_names {
            let result = if *pseudo {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(name)
                    .map(SerialLink::Pseudo)
            } else {
                serialport::new(name, BAUD_RATE)
                    .timeout(Duration::from_millis(200))
                    .open()
                    .map(SerialLink::Physical)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            };
            match result {
                Ok(port) => outputs.push((name.clone(), port)),
                Err(e) => {
                    self.status = format!("Nelze otevřít {name}: {e}");
                    return;
                }
            }
        }
        let input_is_pseudo = std::env::var_os("CASOMIRA_SIM_INPUT_PORT").is_some();
        let input_result = if input_is_pseudo {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.input_name)
                .map(SerialLink::Pseudo)
        } else {
            serialport::new(&self.input_name, BAUD_RATE)
                .timeout(Duration::from_millis(100))
                .open()
                .map(SerialLink::Physical)
                .map_err(|e| std::io::Error::other(e.to_string()))
        };
        let input = match input_result {
            Ok(port) => port,
            Err(e) => {
                self.status = format!("Nelze otevřít {}: {e}", self.input_name);
                return;
            }
        };
        let reader_port = match input.try_clone() {
            Ok(port) => port,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(reader_port);
            loop {
                let mut line = Vec::new();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) => thread::sleep(Duration::from_millis(20)),
                    Ok(_) => {
                        let text = String::from_utf8_lossy(&line).trim().to_string();
                        if tx.send(Ok(text)).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
        });
        self.input = Some(input);
        self.outputs = outputs;
        self.rx = Some(rx);
        self.connected = true;
        self.status = format!("Připojeno: {} → COM20", self.input_name);
        self.log("EVENT", "OUTPUT_PORTS", "COM19 -> COM20");
        self.timer_armed = false;
        self.reset_heat();
    }

    pub(super) fn disconnect(&mut self) {
        self.connected = false;
        self.rx = None;
        self.input = None;
        self.outputs.clear();
        self.started_at = None;
        self.race_elapsed_ms = None;
        self.timer_armed = false;
        self.status = "Odpojeno".into();
    }

    pub(super) fn transmit(&mut self, record: Record, source: Option<&str>) {
        if !self.connected {
            return;
        }
        match record.encode() {
            Ok(line) => {
                for (_, port) in &mut self.outputs {
                    let _ = port.write_all(line.as_bytes());
                }
                self.log("OUT", &line, "");
                if let Some(source) = source {
                    self.log("EVENT", &format!("{source} → {line}"), "");
                }
            }
            Err(e) => self.log("OUT", &e.to_string(), ""),
        }
    }

    pub(super) fn send_timer_byte(&mut self, value: u8) {
        if let Some(port) = self.input.as_mut() {
            let result = port.write_all(&[value]);
            self.log("IN_TX", &value.to_string(), "");
            if let Err(e) = result {
                self.status = e.to_string();
            }
        }
    }
}
