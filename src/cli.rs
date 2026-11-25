use std::env;

#[derive(Debug, Default, Clone)]
pub struct CliArgs {
    pub gui: bool,
    pub help: bool,
    pub list: bool,
    pub duplicate: Option<String>,
    pub export: Option<String>,
    pub output: Option<String>,
    pub import_file: Option<String>,
    pub delete: Option<String>,
    pub force: bool,
    pub title: Option<String>,
    pub workspace: Option<String>,
    pub new_id: bool,
    pub overwrite: bool,
    pub positional: Vec<String>,
}

pub fn parse_args() -> CliArgs {
    let mut args = CliArgs::default();
    let raw: Vec<String> = env::args().skip(1).collect();

    let mut i = 0;
    while i < raw.len() {
        let arg = &raw[i];
        match arg.as_str() {
            "-g" | "--gui" => {
                args.gui = true;
            }
            "-h" | "--help" => {
                args.help = true;
            }
            "-l" | "--list" => {
                args.list = true;
            }
            "-d" | "--duplicate" => {
                if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                    args.duplicate = Some(raw[i + 1].clone());
                    i += 1;
                } else {
                    args.duplicate = Some(String::new());
                }
            }
            "-e" | "--export" => {
                if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                    args.export = Some(raw[i + 1].clone());
                    i += 1;
                } else {
                    args.export = Some(String::new());
                }
            }
            "-o" | "--output" => {
                if i + 1 < raw.len() {
                    args.output = Some(raw[i + 1].clone());
                    i += 1;
                }
            }
            "-i" | "--import" => {
                if i + 1 < raw.len() {
                    args.import_file = Some(raw[i + 1].clone());
                    i += 1;
                }
            }
            "-D" | "--delete" => {
                if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                    args.delete = Some(raw[i + 1].clone());
                    i += 1;
                } else {
                    args.delete = Some(String::new());
                }
            }
            "-f" | "--force" => {
                args.force = true;
            }
            "-t" | "--title" => {
                if i + 1 < raw.len() {
                    args.title = Some(raw[i + 1].clone());
                    i += 1;
                }
            }
            "-w" | "--workspace" => {
                if i + 1 < raw.len() {
                    args.workspace = Some(raw[i + 1].clone());
                    i += 1;
                }
            }
            "--new-id" => {
                args.new_id = true;
            }
            "--overwrite" => {
                args.overwrite = true;
            }
            other => {
                if !other.starts_with('-') {
                    args.positional.push(other.to_string());
                }
            }
        }
        i += 1;
    }

    args
}

pub fn print_help() {
    println!(r#"
agychat (Antigravity Chat Manager)
==================================
Herramienta para ramificar, duplicar, exportar e importar chats
de Google Antigravity entre proyectos o computadoras.

Uso:
  agychat -l | --list
  agychat -d | --duplicate "<CHAT>" [-t | --title "<TITULO>"]
  agychat -e | --export "<CHAT>" [-o | --output <ARCHIVO>]
  agychat -i | --import <ARCHIVO> [-t | --title "<TITULO>"] [-w | --workspace <RUTA>]
  agychat -D | --delete "<CHAT>" [-f | --force]
  agychat -g | --gui
  agychat -h | --help

Opciones:
  -g, --gui                 Inicia la interfaz gráfica de escritorio interactiva en Slint.
  -h, --help                Muestra esta ayuda.
  -l, --list                Lista todas las conversaciones registradas.
  -d, --duplicate CHAT      Duplica un chat en tiempo real por título o ID.
  -e, --export CHAT         Exporta un chat a un archivo portátil (.agychat).
  -i, --import ARCHIVO      Importa un chat desde un archivo .agychat o .tar.gz.
  -D, --delete CHAT         Elimina permanentemente una conversación por título o ID.
  -f, --force               No solicita confirmación antes de eliminar.
  -t, --title TITULO        Título personalizado al duplicar o importar.
  -o, --output ARCHIVO      Ruta del archivo de salida al exportar.
  -w, --workspace RUTA      Ruta del workspace local en la máquina de destino al importar.
      --new-id              Fuerza la creación de un nuevo UUID al importar.
      --overwrite           Sobrescribe el chat si el UUID coincide con uno existente.

Ejemplos:
  1) Abrir interfaz gráfica de escritorio:
     agychat -g
     agychat --gui

  2) Listar chats disponibles en consola:
     agychat -l
     agychat --list

  3) Duplicar con sufijo '(Copia)':
     agychat -d "Analisis Inicial"
     agychat --duplicate "Analisis Inicial"

  4) Duplicar con nuevo título:
     agychat -d "Analisis Inicial" -t "Modulo 2 - Refactor"
     agychat --duplicate "Analisis Inicial" --title "Modulo 2 - Refactor"

  5) Exportar a archivo comprimido portátil (.agychat):
     agychat -e "Analisis Inicial"
     agychat --export "Analisis Inicial" -o ~/Descargas/backup.agychat

  6) Importar en otra máquina:
     agychat -i backup.agychat
     agychat --import backup.agychat -t "Nuevo Titulo" -w ~/Proyectos/mi-repo

  7) Eliminar una conversación:
     agychat -D "Analisis Inicial"
     agychat --delete "Analisis Inicial" --force
"#);
}

pub fn list_conversations_cli() {
    match crate::db::get_all_conversations() {
        Ok(convs) => {
            println!("\n📋 Chats disponibles en Antigravity:");
            if convs.is_empty() {
                println!("  (No hay conversaciones registradas en Antigravity)");
            } else {
                for c in convs {
                    let is_sub = c.is_subagent();
                    let display_title = if c.title.trim().is_empty() {
                        "(Sin título)".to_string()
                    } else {
                        c.title
                    };
                    let tag = if is_sub {
                        if c.agent_name.trim().is_empty() {
                            " [Subagente]".to_string()
                        } else {
                            format!(" [Subagente: {}]", c.agent_name.trim())
                        }
                    } else {
                        String::new()
                    };
                    println!("  • \"{}\"{}  \x1b[90m[{}]\x1b[0m", display_title, tag, c.conversation_id);
                }
            }
            println!();
        }
        Err(e) => {
            eprintln!("❌ Error leyendo base de datos de Antigravity: {}", e);
        }
    }
}
