//! Núcleo reutilizable de Forja GUI.
//!
//! Contiene lógica sin dependencia de xilem/masonry ni del compilador:
//! - [`signals`]: almacén reactivo de variables.
//! - [`debug`]: logging de cambios de estado y callbacks.
//!
//! `forja-gui-rt` consume este crate y re-exporta `signals`.

pub mod debug;
pub mod signals;
