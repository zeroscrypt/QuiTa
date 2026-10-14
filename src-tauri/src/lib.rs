pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
mod platform;

use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            db::init(app.handle())?;

            // Closing the window with the X button hides it instead of
            // quitting; the app stays alive in the tray/menu bar.
            if let Some(win) = app.get_webview_window("main") {
                let win_for_events = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_for_events.hide();
                    }
                });
            }

            // Neither the tray nor the shortcut is worth failing startup for: the
            // window is already open and usable, and the error would surface
            // as a panic after the window stopped closing normally.
            if let Err(err) = platform::tray::setup(app.handle()) {
                eprintln!("QuiTa: tray icon unavailable: {err}");
            }
            platform::shortcuts::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::tab_commands::list_tabs,
            commands::tab_commands::tab_stats,
            commands::tab_commands::create_tab,
            commands::tab_commands::rename_tab,
            commands::tab_commands::delete_tab,
            commands::tab_commands::reorder_tabs,
            commands::app_commands::purge_task,
            commands::app_commands::purge_all_deleted,
            commands::app_commands::reorder_tasks,
            commands::app_commands::reparent_task,
            commands::app_commands::move_task_to_tab,
            commands::app_commands::get_app_info,
            commands::app_commands::db_stats,
            commands::app_commands::vacuum_db,
            commands::app_commands::reverse_root_order,
            commands::app_commands::nest_and_flatten,
            commands::app_commands::undo_nest,
            commands::task_commands::create_task,
            commands::task_commands::rename_task,
            commands::task_commands::toggle_task_completion,
            commands::task_commands::set_task_due_date,
            commands::task_commands::set_task_comment,
            commands::task_commands::soft_delete_task,
            commands::task_commands::restore_task,
            commands::task_commands::move_task,
            commands::task_commands::list_tasks,
            commands::task_commands::list_deleted_tasks,
            commands::task_commands::list_subtasks,
            commands::task_commands::count_subtasks,
            commands::task_commands::purge_completed_tasks,
            commands::task_commands::search_tasks,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            commands::export_commands::export_tasks,
            commands::export_commands::import_tasks,
            commands::window_commands::toggle_window_height,
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuiTa");
}
