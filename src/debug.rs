//! Logging de estado y callbacks del runtime GUI.

/// Retorna true si el modo depuración de GUI está activo.
///
/// Se activa con `FORJA_GUI_DEBUG=1`, `FORJA_DEBUG=1` o `FORJA_GUI_PERF=1`.
/// Antes devolvía `true` fijo: cada escritura de variable (y son muchas por
/// frame) armaba y escribía trazas en stderr, aunque la app GUI no tuviera
/// consola.
pub fn es_depuracion_enabled() -> bool {
    use std::sync::OnceLock;
    static ACTIVO: OnceLock<bool> = OnceLock::new();
    *ACTIVO.get_or_init(|| {
        ["FORJA_GUI_DEBUG", "FORJA_DEBUG", "FORJA_GUI_PERF"]
            .iter()
            .any(|var| {
                std::env::var(var)
                    .map(|v| v != "0" && !v.trim().is_empty())
                    .unwrap_or(false)
            })
    })
}

/// Imprime un mensaje formateado con prefijo si el modo debug está activo.
pub fn depuracion_registrar(category: &str, message: &str) {
    if es_depuracion_enabled() {
        log::debug!("[Forja GUI::{}] {}", category, message);
        eprintln!("🔍 [Forja GUI::{}] {}", category, message);
    }
}

/// Registra el cambio de valor de una variable reactiva.
pub fn log_state_change(
    var_name: &str,
    old_val: Option<&serde_json::Value>,
    nuevo_val: &serde_json::Value,
) {
    if es_depuracion_enabled() {
        match old_val {
            Some(old) => {
                log::info!(
                    "[GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name,
                    old,
                    nuevo_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name, old, nuevo_val
                );
            }
            None => {
                log::info!(
                    "[GUI State] Variable '{}' inicializada: {}",
                    var_name,
                    nuevo_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' inicializada: {}",
                    var_name, nuevo_val
                );
            }
        }
    }
}

/// Registra el inicio de ejecución de un callback.
pub fn registrar_callback_inicio(nombre_fn: &str) {
    if es_depuracion_enabled() {
        log::info!("[GUI Callback] Ejecutando callback '{}'...", nombre_fn);
        eprintln!("▶️ [GUI Callback] Ejecutando callback '{}'...", nombre_fn);
    }
}

/// Registra el éxito de un callback.
pub fn registrar_callback_exito(nombre_fn: &str, resultado: &str) {
    if es_depuracion_enabled() {
        log::info!(
            "[GUI Callback] Callback '{}' completado con éxito: {}",
            nombre_fn,
            resultado
        );
        eprintln!(
            "✅ [GUI Callback] Callback '{}' completado con éxito: {}",
            nombre_fn, resultado
        );
    }
}

/// Registra un error durante la ejecución de un callback o evaluador.
pub fn log_callback_error(nombre_fn: &str, error: &str) {
    log::error!("[GUI Error] Error en callback '{}': {}", nombre_fn, error);
    eprintln!(
        "❌ [GUI Error] Error en callback '{}': {}",
        nombre_fn, error
    );
}

/// Registra una advertencia de conversión de layout.
pub fn registrar_layout_aviso(widget_name: &str, reason: &str) {
    log::warn!("[GUI Layout Warning] Widget '{}': {}", widget_name, reason);
    eprintln!(
        "⚠️ [GUI Layout Warning] Widget '{}': {}",
        widget_name, reason
    );
}
