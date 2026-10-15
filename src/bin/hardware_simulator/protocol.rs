use super::*;

pub(super) fn valid_port_name(s: &str) -> bool {
    s.len() > 3 && s[..3].eq_ignore_ascii_case("COM") && s[3..].parse::<u16>().is_ok()
}

pub(super) fn run(input: &str, output_names: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut input_port = serialport::new(input, 9600)
        .timeout(Duration::from_millis(50))
        .open()?;
    let mut outputs: Vec<(String, Box<dyn SerialPort>)> = output_names
        .iter()
        .map(|name| {
            Ok((
                (*name).to_string(),
                serialport::new(*name, 9600)
                    .timeout(Duration::from_millis(50))
                    .open()?,
            ))
        })
        .collect::<Result<_, serialport::Error>>()?;
    println!(
        "Simulátor je připojen.\nVstup aplikace: {input}; výstupy aplikace: {}\nPříkazy: start | mezičas DRÁHA ČAS_MS | maska MASKA ČAS_HODNOTA | help | konec\nNapříklad: start, potom mezičas 1 12340",
        output_names.join(", ")
    );
    let stdin = io::stdin();
    loop {
        print!("sim> ");
        io::stdout().flush()?;
        let mut command = String::new();
        if stdin.lock().read_line(&mut command)? == 0 {
            break;
        }
        let parts: Vec<_> = command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0].to_lowercase().as_str() {
            "konec" | "quit" | "exit" => break,
            "help" | "nápověda" => println!(
                "start                         odešle startovní událost 255;255\nmezičas DRÁHA ČAS_MS          např. mezičas 2 12340 (12,34 s)\nmaska MASKA ČAS_HODNOTA       přímo odešle masku a syrový čas\nKdyž je čas 255, aplikace použije přesně 255 ms; jinak čas násobí 10."
            ),
            "start" if parts.len() == 1 => send_line(&mut input_port, "255;255")?,
            "mezičas" | "split" if parts.len() == 3 => {
                let lane: u8 = parts[1].parse()?;
                let ms: u32 = parts[2].parse()?;
                if !(1..=8).contains(&lane) {
                    eprintln!("Zadej dráhu v rozsahu 1 až 8.");
                    continue;
                }
                let mut raw = (ms / 10).min(255) as u8;
                if raw == 255 {
                    raw = 256u16 as u8;
                }
                send_line(&mut input_port, &format!("{};{raw}", 1u16 << (lane - 1)))?;
            }
            "maska" if parts.len() == 3 => {
                let mask: u16 = parts[1].parse()?;
                let time: u16 = parts[2].parse()?;
                if mask > 255 || time > 255 {
                    eprintln!("Maska a syrový čas musí být v rozsahu 0 až 255.");
                    continue;
                }
                send_line(&mut input_port, &format!("{mask};{time}"))?;
            }
            _ => {
                eprintln!("Neznámý příkaz. Zadej help.");
                continue;
            }
        }
        let _ = &mut outputs;
    }
    Ok(())
}

fn send_line(port: &mut Box<dyn SerialPort>, line: &str) -> Result<(), Box<dyn std::error::Error>> {
    port.write_all(format!("{line}\r\n").as_bytes())?;
    println!("ODESLÁNO časomíře: {line}");
    Ok(())
}
