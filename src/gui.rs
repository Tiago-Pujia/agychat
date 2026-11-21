use crate::archive::{export_conversation, import_conversation};
use crate::db::{
    delete_conversation, duplicate_conversation, get_all_conversations,
    get_conversation_disk_info, get_total_disk_usage,
};
use crate::ls_client::get_live_ls_info;
use crate::models::ConversationSummary;
use crate::utils::{
    format_bytes, format_relative_time, get_known_projects, resolve_project_and_workspace,
};
use arboard::Clipboard;
use chrono::{DateTime, Utc};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

slint::include_modules!();

#[derive(Default)]
struct GuiState {
    all_conversations: Vec<ConversationSummary>,
    search_query: String,
    active_filter: String,
    sort_column: usize,
    sort_asc: bool,
}

pub fn launch_gui() -> Result<(), slint::PlatformError> {
    let main_window = AppWindow::new()?;
    let state = Arc::new(Mutex::new(GuiState {
        all_conversations: Vec::new(),
        search_query: String::new(),
        active_filter: "all".to_string(),
        sort_column: 3, // Por defecto ordenar por fecha de modificación
        sort_asc: false,
    }));

    // Helper para mostrar Toasts temporales
    let show_toast = {
        let weak = main_window.as_weak();
        move |msg: &str| {
            if let Some(w) = weak.upgrade() {
                w.set_toast_message(msg.into());
                w.set_toast_visible(true);

                let weak_timer = weak.clone();
                slint::Timer::single_shot(Duration::from_millis(3500), move || {
                    if let Some(w2) = weak_timer.upgrade() {
                        w2.set_toast_visible(false);
                    }
                });
            }
        }
    };

    // Helper para recargar y sincronizar la tabla
    let reload_table = {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        move |select_target_cid: Option<String>| {
            if let Some(w) = weak.upgrade() {
                let mut st = state.lock().unwrap();

                // 1. Conectar con LS y actualizar indicador
                if let Some(live) = get_live_ls_info() {
                    w.set_ls_connected(true);
                    w.set_ls_status_text(
                        format!("Antigravity conectado en vivo (puerto HTTPS {})", live.port).into(),
                    );
                } else {
                    w.set_ls_connected(false);
                    w.set_ls_status_text(
                        "Antigravity no detectado en ejecución".into(),
                    );
                }

                // 2. Leer conversaciones de SQLite
                if let Ok(convs) = get_all_conversations() {
                    st.all_conversations = convs;
                }

                // Actualizar contadores de los chips
                let total = st.all_conversations.len() as i32;
                let mut ws_cnt = 0;
                let mut copies_cnt = 0;
                let mut recent_cnt = 0;

                let known_projects = get_known_projects();
                let now = Utc::now();
                for c in &st.all_conversations {
                    let (proj_name, _) = resolve_project_and_workspace(&c.project_id, &c.workspace_uris, &known_projects);
                    if proj_name != "(Sin workspace)" {
                        ws_cnt += 1;
                    }
                    if c.title.to_lowercase().contains("(copia)") {
                        copies_cnt += 1;
                    }
                    if let Ok(parsed) = DateTime::parse_from_rfc3339(&c.last_modified_time) {
                        if (now - parsed.with_timezone(&Utc)).num_days() <= 7 {
                            recent_cnt += 1;
                        }
                    } else if c.step_count > 15 {
                        recent_cnt += 1;
                    }
                }

                w.set_total_count(total);
                w.set_workspace_count(ws_cnt);
                w.set_copies_count(copies_cnt);
                w.set_recent_count(recent_cnt);

                // 3. Filtrar y ordenar
                apply_filter_and_sort(&w, &st, select_target_cid);
            }
        }
    };

    // Actualizar cálculo de disco en background para no retrasar inicio
    {
        let weak = main_window.as_weak();
        thread::spawn(move || {
            let (total_bytes, _) = get_total_disk_usage();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = weak.upgrade() {
                    w.set_disk_summary_text(
                        format!("{} conversaciones · {} en disco", w.get_total_count(), format_bytes(total_bytes)).into(),
                    );
                }
            });
        });
    }

    // Callback Refresh
    {
        let reload = reload_table.clone();
        let toast = show_toast.clone();
        main_window.on_refresh(move || {
            reload(None);
            toast("Tabla actualizada.");
        });
    }

    // Callback Búsqueda
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        main_window.on_search_edited(move |txt| {
            if let Some(w) = weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.search_query = txt.to_string();
                apply_filter_and_sort(&w, &st, None);
            }
        });
    }

    // Callback Filtro Chips
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        main_window.on_filter_selected(move |f| {
            if let Some(w) = weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.active_filter = f.to_string();
                w.set_active_filter(f);
                apply_filter_and_sort(&w, &st, None);
            }
        });
    }

    // Callback Ordenar Columna
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        let toast = show_toast.clone();
        main_window.on_sort_column(move |col| {
            if let Some(w) = weak.upgrade() {
                let mut st = state.lock().unwrap();
                let col_idx = col as usize;
                if st.sort_column == col_idx {
                    st.sort_asc = !st.sort_asc;
                } else {
                    st.sort_column = col_idx;
                    st.sort_asc = true;
                }
                apply_filter_and_sort(&w, &st, None);
                let col_name = match col_idx {
                    0 => "Conversación",
                    1 => "Proyecto",
                    2 => "Pasos",
                    _ => "Modificación",
                };
                toast(&format!("Ordenado por {}", col_name));
            }
        });
    }

    // Callback Selección de Fila
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        main_window.on_row_selected(move |idx, id| {
            if let Some(w) = weak.upgrade() {
                w.set_selected_index(idx);
                w.set_selected_id(id.clone());
                let st = state.lock().unwrap();
                if let Some(c) = st.all_conversations.iter().find(|c| c.conversation_id == id.as_str()) {
                    w.set_selected_title(c.title.clone().into());
                }
            }
        });
    }

    // Función auxiliar para cerrar todos los modales abiertos
    let close_all_modals = |w: &AppWindow| {
        w.set_show_details(false);
        w.set_show_duplicate(false);
        w.set_show_import(false);
        w.set_show_delete(false);
        w.set_show_context_menu(false);
    };

    // Callback Doble Clic (Duplicar)
    {
        let weak = main_window.as_weak();
        let close_fn = close_all_modals.clone();
        main_window.on_row_double_clicked(move |_, _| {
            if let Some(w) = weak.upgrade() {
                close_fn(&w);
                let title = w.get_selected_title();
                w.set_dup_new_title(format!("{} (Copia)", title).into());
                w.set_show_duplicate(true);
            }
        });
    }

    // Callback Clic Derecho (Menú contextual)
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        let close_fn = close_all_modals.clone();
        main_window.on_row_right_clicked(move |idx, id, mx, my| {
            if let Some(w) = weak.upgrade() {
                close_fn(&w);
                w.set_selected_index(idx);
                w.set_selected_id(id.clone());
                let st = state.lock().unwrap();
                if let Some(c) = st.all_conversations.iter().find(|c| c.conversation_id == id.as_str()) {
                    w.set_selected_title(c.title.clone().into());
                }
                w.set_menu_x(mx + 10.0);
                w.set_menu_y(my + 100.0);
                w.set_show_context_menu(true);
            }
        });
    }

    // Acciones: Detalles
    {
        let weak = main_window.as_weak();
        let state = Arc::clone(&state);
        let close_fn = close_all_modals.clone();
        main_window.on_action_details(move || {
            if let Some(w) = weak.upgrade() {
                close_fn(&w);
                let cid = w.get_selected_id();
                let st = state.lock().unwrap();
                if let Some(conv) = st.all_conversations.iter().find(|c| c.conversation_id == cid.as_str()) {
                    let known_projects = get_known_projects();
                    let (proj_name, local_path) = resolve_project_and_workspace(&conv.project_id, &conv.workspace_uris, &known_projects);
                    let disk = get_conversation_disk_info(&conv.conversation_id);

                    w.set_det_project(proj_name.into());
                    w.set_det_path(local_path.into());
                    w.set_det_steps(conv.step_count as i32);
                    w.set_det_date(format_relative_time(&conv.last_modified_time).into());
                    w.set_det_db_size(format_bytes(disk.db_bytes).into());
                    w.set_det_brain_size(format_bytes(disk.brain_bytes).into());
                    w.set_det_brain_files(disk.brain_files as i32);
                    w.set_det_total_size(format_bytes(disk.total_bytes).into());
                    w.set_show_details(true);
                }
            }
        });
    }

    // Acciones: Abrir Carpeta Workspace
    {
        let weak = main_window.as_weak();
        let toast = show_toast.clone();
        main_window.on_open_workspace_folder(move || {
            if let Some(w) = weak.upgrade() {
                let path_str = w.get_det_path().to_string();
                if !path_str.is_empty() && std::path::Path::new(&path_str).exists() {
                    let _ = open::that(&path_str);
                    toast("Carpeta abierta en el explorador");
                }
            }
        });
    }

    // Acciones: Copiar UUID
    {
        let weak = main_window.as_weak();
        let toast = show_toast.clone();
        main_window.on_copy_uuid(move || {
            if let Some(w) = weak.upgrade() {
                let cid = w.get_selected_id().to_string();
                if let Ok(mut cb) = Clipboard::new() {
                    let _ = cb.set_text(&cid);
                    toast("📋 UUID copiado al portapapeles");
                }
            }
        });
    }

    // Acciones: Copiar Ficha
    {
        let weak = main_window.as_weak();
        let toast = show_toast.clone();
        main_window.on_copy_ficha(move || {
            if let Some(w) = weak.upgrade() {
                let ficha = format!(
                    "Título: {}\nUUID: {}\nProyecto: {}\nRuta: {}\nPasos: {}\nAlmacenamiento: {}",
                    w.get_selected_title(),
                    w.get_selected_id(),
                    w.get_det_project(),
                    w.get_det_path(),
                    w.get_det_steps(),
                    w.get_det_total_size()
                );
                if let Ok(mut cb) = Clipboard::new() {
                    let _ = cb.set_text(&ficha);
                    toast("📄 Ficha técnica copiada al portapapeles");
                }
            }
        });
    }

    // Acciones: Duplicar Modal
    {
        let weak = main_window.as_weak();
        let close_fn = close_all_modals.clone();
        main_window.on_action_duplicate(move || {
            if let Some(w) = weak.upgrade() {
                close_fn(&w);
                let title = w.get_selected_title();
                w.set_dup_new_title(format!("{} (Copia)", title).into());
                w.set_show_duplicate(true);
            }
        });
    }

    // Confirmar Duplicar
    {
        let weak = main_window.as_weak();
        let reload = reload_table.clone();
        let toast = show_toast.clone();
        main_window.on_confirm_duplicate(move |new_title| {
            if let Some(w) = weak.upgrade() {
                let cid = w.get_selected_id().to_string();
                let title = new_title.to_string();
                let reload_cb = reload.clone();
                let toast_cb = toast.clone();

                thread::spawn(move || {
                    match duplicate_conversation(&cid, Some(&title)) {
                        Ok(new_id) => {
                            let _ = slint::invoke_from_event_loop(move || {
                                reload_cb(Some(new_id));
                                toast_cb("⚡ Conversación duplicada con éxito");
                            });
                        }
                        Err(e) => {
                            let _ = slint::invoke_from_event_loop(move || {
                                toast_cb(&format!("❌ Error al duplicar: {}", e));
                            });
                        }
                    }
                });
            }
        });
    }

    // Acciones: Exportar
    {
        let weak = main_window.as_weak();
        let toast = show_toast.clone();
        main_window.on_action_export(move || {
            if let Some(w) = weak.upgrade() {
                let cid = w.get_selected_id().to_string();
                let title = w.get_selected_title().to_string();
                let toast_cb = toast.clone();

                let re_clean = regex::Regex::new(r"[^\w\s-]").unwrap();
                let re_spaces = regex::Regex::new(r"[-\s]+").unwrap();
                let mut clean = re_clean.replace_all(&title, "").to_string();
                clean = re_spaces.replace_all(&clean, "_").trim().to_string();
                let default_name = format!("{}.agychat", if clean.is_empty() { "antigravity_chat" } else { &clean });

                let file_opt = rfd::FileDialog::new()
                    .set_file_name(&default_name)
                    .add_filter("Antigravity Chat", &["agychat", "tar.gz"])
                    .save_file();

                if let Some(path) = file_opt {
                    let out_path = path.to_string_lossy().to_string();
                    thread::spawn(move || {
                        match export_conversation(&cid, Some(&out_path)) {
                            Ok(saved) => {
                                let sz_mb = std::fs::metadata(&saved).map(|m| m.len() as f64 / (1024.0 * 1024.0)).unwrap_or(0.0);
                                let _ = slint::invoke_from_event_loop(move || {
                                    toast_cb(&format!("📦 Chat exportado con éxito ({:.2} MB)", sz_mb));
                                });
                            }
                            Err(e) => {
                                let _ = slint::invoke_from_event_loop(move || {
                                    toast_cb(&format!("❌ Error al exportar: {}", e));
                                });
                            }
                        }
                    });
                }
            }
        });
    }

    // Acciones: Eliminar Modal
    {
        let weak = main_window.as_weak();
        let close_fn = close_all_modals.clone();
        main_window.on_action_delete(move || {
            if let Some(w) = weak.upgrade() {
                close_fn(&w);
                w.set_show_delete(true);
            }
        });
    }

    // Confirmar Eliminar
    {
        let reload = reload_table.clone();
        let toast = show_toast.clone();
        main_window.on_confirm_delete(move |cid| {
            let id = cid.to_string();
            let reload_cb = reload.clone();
            let toast_cb = toast.clone();

            thread::spawn(move || {
                match delete_conversation(&id, true) {
                    Ok(true) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            reload_cb(None);
                            toast_cb("🗑️ Conversación eliminada permanentemente");
                        });
                    }
                    Ok(false) => {}
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            toast_cb(&format!("❌ Error al eliminar: {}", e));
                        });
                    }
                }
            });
        });
    }

    // Diálogos de Explorador para Importar
    {
        let weak = main_window.as_weak();
        main_window.on_import_browse_archive(move || {
            if let Some(w) = weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Antigravity Chat (*.agychat, *.tar.gz)", &["agychat", "tar.gz"])
                    .pick_file()
                {
                    w.set_imp_archive_path(path.to_string_lossy().to_string().into());
                }
            }
        });
    }

    {
        let weak = main_window.as_weak();
        main_window.on_import_browse_workspace(move || {
            if let Some(w) = weak.upgrade() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    w.set_imp_target_workspace(folder.to_string_lossy().to_string().into());
                }
            }
        });
    }

    // Confirmar Importar
    {
        let reload = reload_table.clone();
        let toast = show_toast.clone();
        main_window.on_confirm_import(move |archive, title, ws, new_id, overwrite| {
            let arc_path = archive.to_string();
            let c_title = if title.trim().is_empty() { None } else { Some(title.to_string()) };
            let t_ws = if ws.trim().is_empty() { None } else { Some(ws.to_string()) };
            let reload_cb = reload.clone();
            let toast_cb = toast.clone();

            thread::spawn(move || {
                match import_conversation(&arc_path, c_title.as_deref(), t_ws.as_deref(), new_id, overwrite) {
                    Ok(target_id) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            reload_cb(Some(target_id));
                            toast_cb("📥 Chat importado con éxito");
                        });
                    }
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            toast_cb(&format!("❌ Error al importar: {}", e));
                        });
                    }
                }
            });
        });
    }

    // Carga inicial
    reload_table(None);

    main_window.run()
}

fn apply_filter_and_sort(
    window: &AppWindow,
    state: &GuiState,
    select_target_cid: Option<String>,
) {
    let q = state.search_query.to_lowercase().trim().to_string();
    let filter = &state.active_filter;
    let now = Utc::now();
    let known_projects = get_known_projects();

    let mut filtered: Vec<ConversationSummary> = state
        .all_conversations
        .iter()
        .filter(|c| {
            let (proj_name, local_path) = resolve_project_and_workspace(&c.project_id, &c.workspace_uris, &known_projects);

            // Filtro de categoría
            match filter.as_str() {
                "workspace" => {
                    if proj_name == "(Sin workspace)" {
                        return false;
                    }
                }
                "copies" => {
                    if !c.title.to_lowercase().contains("(copia)") {
                        return false;
                    }
                }
                "recent" => {
                    if let Ok(parsed) = DateTime::parse_from_rfc3339(&c.last_modified_time) {
                        if (now - parsed.with_timezone(&Utc)).num_days() > 7 {
                            return false;
                        }
                    } else if c.step_count <= 15 {
                        return false;
                    }
                }
                _ => {}
            }

            // Filtro de texto
            if !q.is_empty() {
                let matches_title = c.title.to_lowercase().contains(&q);
                let matches_proj = proj_name.to_lowercase().contains(&q);
                let matches_path = local_path.to_lowercase().contains(&q);
                let matches_id = c.conversation_id.to_lowercase().contains(&q);
                if !matches_title && !matches_proj && !matches_path && !matches_id {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    // Ordenamiento
    filtered.sort_by(|a, b| {
        let cmp = match state.sort_column {
            0 => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            1 => {
                let (w_a, _) = resolve_project_and_workspace(&a.project_id, &a.workspace_uris, &known_projects);
                let (w_b, _) = resolve_project_and_workspace(&b.project_id, &b.workspace_uris, &known_projects);
                w_a.to_lowercase().cmp(&w_b.to_lowercase())
            }
            2 => a.step_count.cmp(&b.step_count),
            _ => a.last_modified_time.cmp(&b.last_modified_time),
        };
        if state.sort_asc {
            cmp
        } else {
            cmp.reverse()
        }
    });

    let mut new_selected_index = -1;
    let target_id = select_target_cid.or_else(|| {
        let cur = window.get_selected_id();
        if cur.is_empty() { None } else { Some(cur.to_string()) }
    });

    let mut rows: Vec<ConversationRow> = Vec::with_capacity(filtered.len());
    for (idx, item) in filtered.iter().enumerate() {
        if let Some(ref tid) = target_id {
            if &item.conversation_id == tid {
                new_selected_index = idx as i32;
            }
        }

        let (proj_name, local_path) = resolve_project_and_workspace(&item.project_id, &item.workspace_uris, &known_projects);
        let short_id = if item.conversation_id.len() >= 8 {
            format!("{}...", &item.conversation_id[..8])
        } else {
            item.conversation_id.clone()
        };

        rows.push(ConversationRow {
            id: item.conversation_id.clone().into(),
            short_id: short_id.into(),
            title: if item.title.trim().is_empty() { "(Sin título)".into() } else { item.title.clone().into() },
            project: proj_name.into(),
            local_path: local_path.into(),
            steps: item.step_count as i32,
            relative_time: format_relative_time(&item.last_modified_time).into(),
            full_date: item.last_modified_time.clone().into(),
            status_text: item.status.clone().into(),
            is_copy: item.title.to_lowercase().contains("(copia)"),
            is_imported: false,
        });
    }

    let model: Rc<VecModel<ConversationRow>> = Rc::new(VecModel::from(rows));
    window.set_conversations(ModelRc::from(model));

    if new_selected_index >= 0 {
        window.set_selected_index(new_selected_index);
        let sel = &filtered[new_selected_index as usize];
        window.set_selected_id(sel.conversation_id.clone().into());
        window.set_selected_title(sel.title.clone().into());
    } else {
        window.set_selected_index(-1);
        window.set_selected_id("".into());
        window.set_selected_title("".into());
    }
}
