pub mod commands;
pub mod db;
pub mod ssh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::ruleset::list_ruleset,
            commands::ruleset::list_table,
            commands::ruleset::get_rule_summaries,
            commands::ruleset::validate_changeset,
            commands::ruleset::apply_changeset,
            commands::ruleset::confirm_apply,
            commands::ruleset::rollback_apply,
            commands::ruleset::list_snapshots,
            commands::ruleset::create_snapshot,
            commands::ruleset::get_snapshot,
            commands::ruleset::restore_snapshot,
            commands::ruleset::delete_snapshot,
            commands::templates::list_templates,
            commands::templates::render_template,
            commands::monitor::get_monitor_socket_path,
            commands::monitor::start_monitoring,
            commands::monitor::stop_monitoring,
            commands::monitor::read_monitor_sample,
            commands::hosts::list_hosts,
        ])
        .setup(|app| {
            let _ = app.handle();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running palisade");
}
