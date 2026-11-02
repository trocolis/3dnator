//! Abre arquivos exportados em qualquer fatiador: pelo aplicativo padrão do sistema ou por um
//! comando escrito pelo usuário. O comando é executado sem shell, com argumentos separados.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Marca a posição dos arquivos no comando; sem ela, eles entram no final.
const FILE_MARK: &str = "{arquivo}";

/// Separa por espaços, respeitando aspas: caminhos de programas podem ter espaços.
fn split(command: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut started) = (false, false);
    for character in command.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err("Aspas sem fechamento no comando do fatiador.".into());
    }
    if started {
        parts.push(current);
    }
    Ok(parts)
}

/// Programa e argumentos do comando configurado para `files`.
fn arguments(command: &str, files: &[PathBuf]) -> Result<Vec<OsString>, String> {
    let parts = split(command)?;
    let mut result = Vec::with_capacity(parts.len() + files.len());
    let mut placed = false;
    for part in parts {
        if part == FILE_MARK {
            result.extend(files.iter().map(OsString::from));
            placed = true;
        } else {
            result.push(OsString::from(part));
        }
    }
    if result.is_empty() {
        return Err("Comando do fatiador vazio.".into());
    }
    if !placed {
        result.extend(files.iter().map(OsString::from));
    }
    Ok(result)
}

fn run(program: &OsString, arguments: &[OsString]) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{}: {e}", Path::new(program).display()))?;
    // O fatiador continua aberto depois da exportação; a thread só evita um processo zumbi.
    std::thread::spawn(move || child.wait());
    Ok(())
}

fn default_opener() -> (&'static str, &'static [&'static str]) {
    if cfg!(target_os = "windows") {
        ("rundll32", &["url.dll,FileProtocolHandler"])
    } else if cfg!(target_os = "macos") {
        ("open", &[])
    } else {
        ("xdg-open", &[])
    }
}

pub(super) fn open(command: &str, files: &[PathBuf]) -> Result<(), String> {
    if command.trim().is_empty() {
        let (program, fixed) = default_opener();
        for file in files {
            let mut arguments = fixed.iter().map(OsString::from).collect::<Vec<_>>();
            arguments.push(file.into());
            run(&program.into(), &arguments)?;
        }
        return Ok(());
    }
    let arguments = arguments(command, files)?;
    let (program, rest) = arguments
        .split_first()
        .ok_or("Comando do fatiador vazio.")?;
    run(program, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<PathBuf> {
        vec![PathBuf::from("/tmp/caixa com espaço.3mf")]
    }
    fn text(arguments: Vec<OsString>) -> Vec<String> {
        arguments
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn files_go_to_the_mark_or_to_the_end_as_single_arguments() -> Result<(), String> {
        assert_eq!(
            text(arguments("fatiador --carregar {arquivo} --sair", &files())?),
            [
                "fatiador",
                "--carregar",
                "/tmp/caixa com espaço.3mf",
                "--sair"
            ]
        );
        assert_eq!(
            text(arguments("fatiador", &files())?),
            ["fatiador", "/tmp/caixa com espaço.3mf"]
        );
        Ok(())
    }

    #[test]
    fn quotes_keep_program_paths_together_and_invalid_commands_are_refused() -> Result<(), String> {
        assert_eq!(
            text(arguments(r#""/opt/meu fatiador/app" -x"#, &files())?)[..2],
            ["/opt/meu fatiador/app", "-x"]
        );
        assert!(arguments(r#""sem fim"#, &files()).is_err());
        assert!(arguments("   ", &[]).is_err());
        // Sem shell, metacaracteres são texto comum.
        assert_eq!(
            text(arguments("fatiador ; rm", &[])?),
            ["fatiador", ";", "rm"]
        );
        Ok(())
    }

    /// Um programa de verdade recebe o caminho com espaços como um único argumento.
    #[cfg(unix)]
    #[test]
    fn the_configured_program_really_runs_with_the_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let target = directory.path().join("modelo com espaço.3mf");
        open("touch {arquivo}", std::slice::from_ref(&target))?;
        for _ in 0..200 {
            if target.exists() {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err("O programa não recebeu o arquivo".into())
    }

    #[test]
    fn missing_programs_are_reported() {
        assert!(open("programa-que-nao-existe-3dnator", &files()).is_err());
    }
}
