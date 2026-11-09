mod archive;
mod cli;
mod db;
mod gui;
mod ls_client;
mod models;
mod paths;
mod utils;

use std::env;
use std::process;

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let args = cli::parse_args();

    // 1. Modo GUI (-g o --gui)
    if args.gui {
        if let Err(e) = gui::launch_gui() {
            eprintln!("❌ Error iniciando interfaz gráfica en Slint: {}", e);
            process::exit(1);
        }
        return;
    }

    // 2. Sin argumentos o -h / --help
    if args.help || raw_args.len() <= 1 {
        cli::print_help();
        return;
    }

    // 3. Listar conversaciones (-l o --list)
    if args.list {
        cli::list_conversations_cli();
        return;
    }

    // 4. Eliminar conversación (-D o --delete)
    if let Some(mut target) = args.delete {
        let mut pos = args.positional.clone();
        if target.trim().is_empty() && !pos.is_empty() {
            target = pos.remove(0);
        }
        if target.trim().is_empty() {
            eprintln!("❌ Error: Debes especificar el nombre o ID del chat a eliminar.");
            eprintln!("   Uso: agychat -D \"<Nombre del Chat>\" [-f | --force]");
            process::exit(1);
        }
        match db::delete_conversation(&target, args.force) {
            Ok(true) => {
                println!("✅ Conversación eliminada correctamente.");
            }
            Ok(false) => {}
            Err(e) => {
                eprintln!("❌ Error al eliminar conversación: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // 5. Duplicar conversación (-d o --duplicate)
    if let Some(mut target) = args.duplicate {
        let mut pos = args.positional.clone();
        if target.trim().is_empty() && !pos.is_empty() {
            target = pos.remove(0);
        }
        if target.trim().is_empty() {
            eprintln!("❌ Error: Debes especificar el nombre o ID del chat a duplicar.");
            eprintln!("   Uso: agychat -d \"<Nombre del Chat>\" [-t \"<Nuevo Título>\"]");
            process::exit(1);
        }
        let custom_title = args.title.clone().or_else(|| pos.first().cloned());
        match db::duplicate_conversation(&target, custom_title.as_deref()) {
            Ok(new_id) => {
                println!("\n✅ ¡Listo! El chat ha sido duplicado con éxito.");
                println!("   UUID generado: [{}]", new_id);
                println!("   Ya aparece visible en tu barra lateral de Antigravity.\n");
            }
            Err(e) => {
                eprintln!("\n❌ Error al duplicar chat: {}\n", e);
                process::exit(1);
            }
        }
        return;
    }

    // 6. Exportar conversación (-e o --export)
    if let Some(mut target) = args.export {
        let mut pos = args.positional.clone();
        let mut output_file = args.output.clone();
        if target.trim().is_empty() && !pos.is_empty() {
            target = pos.remove(0);
        }
        if output_file.is_none() && !pos.is_empty() {
            output_file = Some(pos.remove(0));
        }
        if target.trim().is_empty() {
            eprintln!("❌ Error: Debes especificar el nombre o ID del chat a exportar.");
            eprintln!("   Uso: agychat -e \"<Nombre del Chat>\" [-o archivo.agychat]");
            process::exit(1);
        }
        match archive::export_conversation(&target, output_file.as_deref()) {
            Ok(path) => {
                let sz_mb = std::fs::metadata(&path)
                    .map(|m| m.len() as f64 / (1024.0 * 1024.0))
                    .unwrap_or(0.0);
                println!("\n✅ ¡Exportación completada con éxito! ({:.2} MB)", sz_mb);
                println!("   Archivo: {}", path.display());
                println!("   Para importarlo en otra máquina ejecuta:");
                println!("   agychat -i \"{}\"\n", path.display());
            }
            Err(e) => {
                eprintln!("\n❌ Error al exportar: {}\n", e);
                process::exit(1);
            }
        }
        return;
    }

    // 7. Importar conversación (-i o --import)
    if let Some(import_file) = args.import_file {
        let custom_title = args.title.clone().or_else(|| args.positional.first().cloned());
        match archive::import_conversation(
            &import_file,
            custom_title.as_deref(),
            args.workspace.as_deref(),
            args.new_id,
            args.overwrite,
        ) {
            Ok(new_id) => {
                println!("\n✅ ¡Listo! El chat ha sido importado exitosamente.");
                println!("   UUID: [{}]", new_id);
                println!("   Ya aparece visible en tu barra lateral de Antigravity.\n");
            }
            Err(e) => {
                eprintln!("\n❌ Error al importar: {}\n", e);
                process::exit(1);
            }
        }
        return;
    }

    // 8. Posicional por defecto (duplicar si se pasa nombre directamente)
    if !args.positional.is_empty() {
        let target = &args.positional[0];
        let custom_title = args.title.clone().or_else(|| {
            if args.positional.len() > 1 {
                Some(args.positional[1].clone())
            } else {
                None
            }
        });
        match db::duplicate_conversation(target, custom_title.as_deref()) {
            Ok(new_id) => {
                println!("\n✅ ¡Listo! El chat ha sido duplicado con éxito.");
                println!("   UUID generado: [{}]", new_id);
                println!("   Ya aparece visible en tu barra lateral de Antigravity.\n");
            }
            Err(e) => {
                eprintln!("\n❌ Error al duplicar chat: {}\n", e);
                process::exit(1);
            }
        }
        return;
    }

    cli::print_help();
}
