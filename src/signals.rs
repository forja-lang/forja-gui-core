// Forja GUI Runtime — Signals/Streams reactivos
// Reemplaza HashMap<String, ValorGUI> con un sistema reactivo
// que permite actualización granular de widgets vía generación counters.
//
// Arquitectura:
//   Signal<T>      → valor observable con detección de cambios O(1)
//   VariableStore  → contenedor de signals nombradas
//   Stream<T>      → canal de eventos (para futuras extensiones)

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

// ─── Signal ──────────────────────────────────────────────────────────
//
/// Un valor reactivo con seguimiento de cambios mediante un contador
/// de generación atómico. Cada escritura incrementa la generación,
/// permitiendo a los consumidores detectar cambios sin comparar valores.
///
/// # Reactividad
/// Los widgets pueden leer `generation()` antes y después de una operación
/// para determinar si hubo cambios sin necesidad de comparar el valor.
///
/// # Thread Safety
/// `Signal<T>` es `Send + Sync` cuando `T: Send + Sync`.
#[derive(Debug)]
pub struct Signal<T> {
    inner: Arc<RwLock<SignalInner<T>>>,
}

#[derive(Debug)]
struct SignalInner<T> {
    valor: T,
    generation: u64,
}

impl<T> Signal<T> {
    /// Crea una nueva señal con el valor inicial y generación 0.
    pub fn nuevo(valor: T) -> Self {
        Signal {
            inner: Arc::new(RwLock::new(SignalInner {
                valor,
                generation: 0,
            })),
        }
    }

    /// Obtiene el valor actual (clonado) y su generación.
    /// Requiere `T: Clone`.
    pub fn obtener(&self) -> (T, u64)
    where
        T: Clone,
    {
        let guard = self.inner.read().expect("Signal::get lock poisoned");
        (guard.valor.clone(), guard.generation)
    }

    /// Escribe un nuevo valor, incrementa la generación y la retorna.
    pub fn escribir(&self, valor: T) -> u64 {
        let mut guard = self.inner.write().expect("Signal::write lock poisoned");
        guard.valor = valor;
        guard.generation += 1;
        guard.generation
    }

    /// Obtiene la generación actual (sin leer el valor).
    pub fn generation(&self) -> u64 {
        self.inner
            .read()
            .expect("Signal::generation lock poisoned")
            .generation
    }

    /// Reemplaza el valor y retorna el valor anterior junto con la nueva generación.
    pub fn replace(&self, valor: T) -> (T, u64)
    where
        T: Clone,
    {
        let mut guard = self.inner.write().expect("Signal::replace lock poisoned");
        let old = guard.valor.clone();
        guard.valor = valor;
        guard.generation += 1;
        (old, guard.generation)
    }

    /// Actualiza el valor mediante una función, incrementa generación.
    pub fn actualizar<F>(&self, f: F) -> u64
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.inner.write().expect("Signal::update lock poisoned");
        f(&mut guard.valor);
        guard.generation += 1;
        guard.generation
    }
}

impl<T: PartialEq> Signal<T> {
    /// Escribe solo si el nuevo valor es diferente al actual.
    /// Retorna `true` si hubo cambio (y se incrementó la generación).
    pub fn escribir_if_changed(&self, valor: T) -> bool {
        let mut guard = self
            .inner
            .write()
            .expect("Signal::write_if_changed lock poisoned");
        if guard.valor != valor {
            guard.valor = valor;
            guard.generation += 1;
            true
        } else {
            false
        }
    }
}

impl<T: Clone> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ─── Stream ──────────────────────────────────────────────────────────
//
/// Un canal de eventos simple para comunicación reactiva.
/// Cada `emit()` envía un valor y notifica a los consumidores.
///
/// Placeholder para futura implementación con multicanal.
#[derive(Debug)]
pub struct Stream<T> {
    inner: Arc<RwLock<StreamInner<T>>>,
}

#[derive(Debug)]
struct StreamInner<T> {
    valor: Option<T>,
    generation: u64,
}

impl<T> Stream<T> {
    /// Crea un nuevo stream vacío.
    pub fn nuevo() -> Self {
        Stream {
            inner: Arc::new(RwLock::new(StreamInner {
                valor: None,
                generation: 0,
            })),
        }
    }

    /// Emite un valor al stream, incrementando la generación.
    pub fn emitir(&self, valor: T) -> u64 {
        let mut guard = self.inner.write().expect("Stream::emit lock poisoned");
        guard.valor = Some(valor);
        guard.generation += 1;
        guard.generation
    }

    /// Obtiene el último valor emitido (si hay) y su generación.
    pub fn peek(&self) -> (Option<T>, u64)
    where
        T: Clone,
    {
        let guard = self.inner.read().expect("Stream::peek lock poisoned");
        (guard.valor.clone(), guard.generation)
    }

    /// Obtiene la generación actual.
    pub fn generation(&self) -> u64 {
        self.inner
            .read()
            .expect("Stream::generation lock poisoned")
            .generation
    }
}

impl<T> Default for Stream<T> {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl<T: Clone> Clone for Stream<T> {
    fn clone(&self) -> Self {
        Stream {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ─── ValorReactivio (enum wrapper) ──────────────────────────────────
//
/// Un valor que puede ser Signal (reactivo) o fijo.
/// Útil para parámetros de widgets que pueden ser estáticos o dinámicos.
#[derive(Debug, Clone)]
pub enum ValorReactivio<T: Clone> {
    Fijo(T),
    Signal(Signal<T>),
}

impl<T: Clone> ValorReactivio<T> {
    /// Obtiene el valor actual y su generación (0 si es fijo).
    pub fn obtener(&self) -> (T, u64) {
        match self {
            ValorReactivio::Fijo(val) => (val.clone(), 0),
            ValorReactivio::Signal(sig) => sig.obtener(),
        }
    }

    /// Obtiene la generación actual (0 si es fijo).
    pub fn generation(&self) -> u64 {
        match self {
            ValorReactivio::Fijo(_) => 0,
            ValorReactivio::Signal(sig) => sig.generation(),
        }
    }

    /// Crea un ValorReactivio desde un valor fijo.
    pub fn fijo(val: T) -> Self {
        ValorReactivio::Fijo(val)
    }

    /// Crea un ValorReactivio desde una signal.
    pub fn reactivo(signal: Signal<T>) -> Self {
        ValorReactivio::Signal(signal)
    }
}

impl<T: Clone> From<T> for ValorReactivio<T> {
    fn from(val: T) -> Self {
        ValorReactivio::Fijo(val)
    }
}

impl<T: Clone> From<Signal<T>> for ValorReactivio<T> {
    fn from(signal: Signal<T>) -> Self {
        ValorReactivio::Signal(signal)
    }
}

// ─── VariableStore ───────────────────────────────────────────────────
//
/// Reemplaza `HashMap<String, ValorGUI>` con un mapa de señales reactivas.
///
/// Cada variable es una `Signal<serde_json::Value>` independiente.
/// Los widgets pueden leer el valor y su generación para decidir
/// si necesitan re-renderizarse.
///
/// # Reactividad
/// ```ignore
/// let store = VariableStore::new();
/// store.fijar("contador", json!(42));
/// let gen = store.generation("contador");
/// // ... algo cambia ...
/// if store.generation("contador") != gen {
///     // el valor cambió, hay que re-renderizar
/// }
/// ```
///
/// # Thread Safety
/// `VariableStore` es `Send + Sync`.
#[derive(Debug, Clone)]
pub struct VariableAlmacen {
    signals: Arc<RwLock<HashMap<String, Signal<serde_json::Value>>>>,
    /// Generación global que se incrementa en cada escritura.
    /// Útil para invalidación completa del árbol.
    global_gen: Arc<AtomicU64>,
}

impl VariableAlmacen {
    /// Crea un nuevo VariableStore vacío.
    pub fn nuevo() -> Self {
        VariableAlmacen {
            signals: Arc::new(RwLock::new(HashMap::new())),
            global_gen: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Obtiene o crea la señal para una variable.
    fn ensure_signal(&self, name: &str) -> Signal<serde_json::Value> {
        let mut mapa = self
            .signals
            .write()
            .expect("VariableStore::ensure_signal lock poisoned");
        mapa.entry(name.to_string())
            .or_insert_with(|| Signal::nuevo(serde_json::Value::Null))
            .clone()
    }

    /// Obtiene la señal para una variable, si existe.
    fn obtener_signal(&self, name: &str) -> Option<Signal<serde_json::Value>> {
        let mapa = self
            .signals
            .read()
            .expect("VariableStore::get_signal lock poisoned");
        mapa.get(name).cloned()
    }

    /// Lee el valor de una variable.
    /// Si no existe, retorna `None`.
    pub fn obtener(&self, name: &str) -> Option<serde_json::Value> {
        self.obtener_signal(name).map(|sig| sig.obtener().0)
    }

    /// Escribe un valor en una variable, incrementa su generación
    /// y la generación global. Retorna la generación de la señal.
    pub fn fijar(&self, name: &str, valor: serde_json::Value) -> u64 {
        let old_val = self.obtener(name);
        let signal = self.ensure_signal(name);
        let gen = signal.escribir(valor.clone());
        self.global_gen.fetch_add(1, Ordering::Release);
        crate::debug::log_state_change(name, old_val.as_ref(), &valor);
        gen
    }

    /// Obtiene la generación de una variable específica.
    /// Si no existe, retorna 0.
    pub fn generation(&self, name: &str) -> u64 {
        self.obtener_signal(name)
            .map(|sig| sig.generation())
            .unwrap_or(0)
    }

    /// Obtiene la generación global (se incrementa en cada escritura).
    pub fn global_generation(&self) -> u64 {
        self.global_gen.load(Ordering::Acquire)
    }

    /// Verifica si una variable existe.
    pub fn contains(&self, name: &str) -> bool {
        let mapa = self
            .signals
            .read()
            .expect("VariableStore::contains lock poisoned");
        mapa.contains_key(name)
    }

    /// Obtiene el número de variables almacenadas.
    pub fn longitud(&self) -> usize {
        let mapa = self
            .signals
            .read()
            .expect("VariableStore::len lock poisoned");
        mapa.len()
    }

    /// Retorna true si no hay variables.
    pub fn es_vacio(&self) -> bool {
        self.longitud() == 0
    }

    /// Obtiene un snapshot de todas las variables (para depuración).
    pub fn snapshot(&self) -> HashMap<String, serde_json::Value> {
        let mapa = self
            .signals
            .read()
            .expect("VariableStore::snapshot lock poisoned");
        mapa.iter()
            .map(|(k, sig)| (k.clone(), sig.obtener().0))
            .collect()
    }

    /// Inicializa múltiples variables desde un iterador.
    pub fn inicializar_from<I: IntoIterator<Item = (String, serde_json::Value)>>(&self, iter: I) {
        let mut mapa = self
            .signals
            .write()
            .expect("VariableStore::init_from lock poisoned");
        for (name, valor) in iter {
            let entrada = mapa
                .entry(name)
                .or_insert_with(|| Signal::nuevo(serde_json::Value::Null));
            entrada.escribir(valor);
        }
    }
}

impl Default for VariableAlmacen {
    fn default() -> Self {
        Self::nuevo()
    }
}

// ─── ReactiveContext ─────────────────────────────────────────────────
//
/// Contexto reactivo para el árbol de widgets.
///
/// Mantiene un registro de qué señales se leyeron durante el render actual,
/// permitiendo al sistema optimizar las actualizaciones.
#[derive(Debug, Clone)]
pub struct ReactiveCtx {
    /// Variables leídas durante el último render
    dependencies: Arc<RwLock<Vec<String>>>,
}

impl ReactiveCtx {
    /// Crea un nuevo contexto reactivo.
    pub fn nuevo() -> Self {
        ReactiveCtx {
            dependencies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Registra que una variable fue leída.
    pub fn track_dependency(&self, name: &str) {
        if let Ok(mut deps) = self.dependencies.write() {
            if !deps.contains(&name.to_string()) {
                deps.push(name.to_string());
            }
        }
    }

    /// Obtiene las dependencias registradas.
    pub fn obtener_dependencies(&self) -> Vec<String> {
        self.dependencies
            .read()
            .map(|deps| deps.clone())
            .unwrap_or_default()
    }

    /// Limpia las dependencias registradas.
    pub fn limpiar(&self) {
        if let Ok(mut deps) = self.dependencies.write() {
            deps.clear();
        }
    }
}

impl Default for ReactiveCtx {
    fn default() -> Self {
        Self::nuevo()
    }
}

// ─── Reactive Operations (helpers para widgets) ─────────────────────
//
/// Lee un valor reactivo con tracking de dependencia.
pub fn leer_variable(
    almacen: &VariableAlmacen,
    name: &str,
    _ctx: Option<&ReactiveCtx>,
) -> Option<serde_json::Value> {
    if let Some(ctx) = _ctx {
        ctx.track_dependency(name);
    }
    almacen.obtener(name)
}

/// Escribe un valor reactivo.
pub fn escribir_variable(almacen: &VariableAlmacen, name: &str, valor: serde_json::Value) -> u64 {
    almacen.fijar(name, valor)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_signal_basic() {
        let sig = Signal::nuevo(42i64);
        assert_eq!(sig.generation(), 0);

        let (_, gen) = sig.obtener();
        assert_eq!(gen, 0);

        let nuevo_generacion = sig.escribir(100);
        assert!(nuevo_generacion > 0);
        assert_eq!(sig.generation(), nuevo_generacion);

        let (val, _) = sig.obtener();
        assert_eq!(val, 100);
    }

    #[test]
    fn test_signal_escribir_if_changed() {
        let sig = Signal::nuevo(42i64);
        assert!(!sig.escribir_if_changed(42)); // mismo valor
        assert_eq!(sig.generation(), 0);

        assert!(sig.escribir_if_changed(100)); // valor diferente
        assert_eq!(sig.generation(), 1);
    }

    #[test]
    fn test_signal_actualizar() {
        let sig = Signal::nuevo(vec![1, 2, 3]);
        sig.actualizar(|v| v.push(4));
        assert_eq!(sig.obtener().0, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_signal_hilo_safety() {
        let sig = Signal::nuevo(0i64);
        let sig2 = sig.clone();

        std::thread::spawn(move || {
            sig2.escribir(42);
        })
        .join()
        .unwrap();

        assert_eq!(sig.obtener().0, 42);
    }

    #[test]
    fn test_variable_almacen() {
        let almacen = VariableAlmacen::nuevo();

        // set/get
        almacen.fijar("nombre", json!("Juan"));
        assert_eq!(almacen.obtener("nombre"), Some(json!("Juan")));

        // generation
        let gen1 = almacen.generation("nombre");
        assert!(gen1 > 0);

        almacen.fijar("nombre", json!("María"));
        let gen2 = almacen.generation("nombre");
        assert!(gen2 > gen1);

        // contains
        assert!(almacen.contains("nombre"));
        assert!(!almacen.contains("no_existe"));

        // non-existent
        assert_eq!(almacen.obtener("no_existe"), None);
        assert_eq!(almacen.generation("no_existe"), 0);
    }

    #[test]
    fn test_variable_almacen_inicializar() {
        let almacen = VariableAlmacen::nuevo();
        let mut data = HashMap::new();
        data.insert("a".to_string(), json!(1));
        data.insert("b".to_string(), json!("hola"));

        almacen.inicializar_from(data.clone());
        assert_eq!(almacen.obtener("a"), data.get("a").cloned());
        assert_eq!(almacen.obtener("b"), data.get("b").cloned());
    }

    #[test]
    fn test_valor_reactivio() {
        let sig = Signal::nuevo(42i64);
        let fijo: ValorReactivio<i64> = ValorReactivio::Fijo(10);
        let reactivo: ValorReactivio<i64> = ValorReactivio::Signal(sig);

        assert_eq!(fijo.obtener(), (10, 0));
        assert_eq!(reactivo.generation(), 0);

        if let ValorReactivio::Signal(s) = &reactivo {
            s.escribir(99);
        }
        assert_eq!(reactivo.obtener().0, 99);
    }

    #[test]
    fn test_variable_almacen_hilo_safety() {
        let almacen = VariableAlmacen::nuevo();
        let almacen_clonar = almacen.clone();

        let handle = std::thread::spawn(move || {
            almacen_clonar.fijar("x", json!(42));
        });

        handle.join().unwrap();
        assert_eq!(almacen.obtener("x"), Some(json!(42)));
    }

    #[test]
    fn test_almacen_snapshot_and_longitud() {
        let almacen = VariableAlmacen::nuevo();
        assert!(almacen.es_vacio());
        assert_eq!(almacen.longitud(), 0);

        almacen.fijar("x", json!(1));
        almacen.fijar("y", json!(2));

        assert_eq!(almacen.longitud(), 2);
        assert!(!almacen.es_vacio());

        let snap = almacen.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap.get("x"), Some(&json!(1)));
    }

    #[test]
    fn test_signal_clone_independent_gen() {
        let sig = Signal::nuevo("original".to_string());
        let cloned = sig.clone();

        // cloned ve el mismo valor inicial
        assert_eq!(cloned.obtener().0, "original".to_string());

        // escribir en original incrementa la gen para todos
        sig.escribir("modificado".to_string());
        assert_eq!(cloned.obtener().0, "modificado".to_string());

        // generation se comparte porque apunta al mismo Arc
        assert_eq!(sig.generation(), cloned.generation());
    }

    #[test]
    fn test_global_generation() {
        let almacen = VariableAlmacen::nuevo();
        let g0 = almacen.global_generation();

        almacen.fijar("a", json!(1));
        assert!(almacen.global_generation() > g0);

        let g1 = almacen.global_generation();
        almacen.fijar("b", json!(2));
        assert!(almacen.global_generation() > g1);
    }

    #[test]
    fn test_stream_basic() {
        let stream = Stream::nuevo();
        assert_eq!(stream.generation(), 0);

        stream.emitir("hola");
        assert!(stream.generation() > 0);

        stream.emitir("mundo");
        assert!(stream.generation() > 0);
    }
}
