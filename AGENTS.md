# AGENTS.md — MielFTP

Guía para agentes de Cursor (y asistentes similares) que trabajan en este repositorio. Léela al inicio de cada sesión.

## Qué es este proyecto

**MielFTP** es un cliente FTP de escritorio en **Rust**, UI con **Iced 0.14**, inspirado en **FileZilla** con estética **shadcn / Cursor / Claude** (tema oscuro, acento miel `#c9a84c`).

| Documento | Uso |
|-----------|-----|
| [README.md](README.md) | Uso, build, roadmap con ✅/⬜ |
| [PLAN.md](PLAN.md) | Auditoría, fases, decisiones de diseño, deuda técnica |

**No edites** `PLAN.md` salvo que el usuario pida actualizar el plan explícitamente.

## Objetivo de producto

Paridad funcional progresiva con FileZilla, sin sacrificar un binario Rust idiomático ni una UI minimalista. Priorizar **correctness** y **cambios pequeños** sobre abstracciones prematuras.

## Stack

- Rust **edition 2024**, `iced` + feature `tokio`
- FTP: `suppaftp` (async, `async-native-tls` en Cargo — FTPS aún no implementado)
- Persistencia: `serde_json` + `dirs`; contraseñas: `keyring` + `secrecy`
- Errores: `thiserror`; logs: `tracing` / `tracing-subscriber`

## Arquitectura (capas)

```
main.rs          → arranque, tracing_init, iced::application
app.rs           → boot / update / view (orquestador; sin re-exportar models)
models/          → State, Message, Connection, FtpEntry, FtpLog, FtpTaskResult
ftp/client.rs    → operaciones de red; devuelve FtpResponse { result, log }
config/store.rs  → sites.json atómico (sin passwords)
ui/              → vistas puras (sidebar, file_panel, modals, theme)
error.rs         → AppError, AppErrorMsg (Arc para Clone en Message)
```

**Flujo iced:** `Message` → `update(&mut State)` → `Task<Message>` → `view(&State)`.

**FTP:** `State.ftp_manager` (`FtpSessionManager`) mantiene un stream por sitio; ver `src/ftp/manager.rs`.

**Regla de imports:** `ui/` y `ftp/` importan desde `crate::models::*`, **nunca** desde `crate::app`.

## Estado actual vs objetivo

| Hecho (Fases 1–4) | Pendiente futuro |
|-------------------|------------------|
| Site Manager + keyring, quickconnect, bookmarks | Cobertura de tests ampliada |
| Sesión persistente, streaming, cola, progreso concurrente | Host key verification SFTP estricta |
| UX FileZilla + sync/compare + edición remota | Watcher auto re-upload en edición remota |
| FTPS/SFTP, throttling, filtros, toasts, skeleton | |
| Log FTP **opcional** (toggle), `AppError` tipado | |

## Convenciones de código

### Rust

- Preferir `AppResult<T>` / `AppError` en lógica; en `Message` usar `AppErrorMsg` o `FtpTaskResult<T>` (ya `Clone`).
- **Match exhaustivo** en `update` — prohibido `_ => Task::none()` que oculte variantes nuevas.
- Validar conexiones con `Connection::try_new`; passwords solo vía `SecretString` y `expose_secret()` en login FTP.
- Rutas **locales:** `PathBuf::push`. Rutas **remotas:** POSIX (`format!` o join FTP).
- `impl Debug` manual en `Connection` — password `[REDACTED]`.
- Sin `unsafe` salvo justificación documentada.
- Warnings: el proyecto pasa `cargo clippy -- -D warnings`.

### iced / UI

- Colores y tokens en [`src/ui/theme.rs`](src/ui/theme.rs) — no hardcodear hex fuera del theme salvo excepción local.
- Textos UI en **español** (botones, labels, errores de validación).
- Modales: patrón en `connection_modal.rs` (overlay + card).
- **Log FTP:** oculto por defecto; toggle en `transfer_bar` / `ftp_log_panel` — **no** hacer el log siempre visible.

### FTP

- Parser de listados: `FtpEntry::from_list_line` → `suppaftp::list::File` (no reimplementar LIST).
- Operaciones async devuelven `FtpResponse<T> { result, log }`; el reducer aplica log con `apply_ftp_log`.
- Líneas de log estilo comando: `> CONNECT`, `> USER`, `PASS ***`, `> CWD`, etc.

### Config

- `~/.config/miel-ftp/sites.json` (o `%APPDATA%` en Windows).
- Escritura: `sites.json.tmp` + `rename` (atómica).
- Password **nunca** en JSON.

## Cómo trabajar (agentes)

1. **Lee** README roadmap + PLAN fase relevante antes de implementar features grandes.
2. **Alcance mínimo:** un diff enfocado; no refactorizar archivos no relacionados.
3. **Convenciones existentes:** copia el estilo del módulo vecino (nombres, spacing iced, patrones `Task::perform`).
4. **Verificar:** `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.
5. **No commits** salvo petición explícita del usuario.
6. **No editar** el plan en `.cursor/plans/` del usuario; el plan del repo es `PLAN.md`.
7. **Comunicación con el usuario:** español, técnico y directo; citar código como ` ```start:end:path `.

### Añadir un `Message` nuevo

1. Variante en `models/message.rs`
2. Brazo en `app::update` (sin catch-all)
3. Origen en `ui/` o `Task::perform` / `Subscription`
4. Si es FTP remoto: considerar `FtpTaskResult` + líneas en `ftp/client.rs`

### Añadir UI

1. Módulo en `src/ui/`
2. Registrar en `ui/mod.rs`
3. Componer desde `app::view`; estado en `State` si hace falta persistencia

## Diseño visual

- Base `#111111`, surface `#161616`, texto `#e2e2e2` / muted `#888888`
- Acento miel `#c9a84c`, success `#4caf7d`, danger `#e05c5c`
- Tipografía compacta (10–13px en tablas y chrome)
- Evitar nuevos emojis en UI si se puede; preferir texto o SVG futuro (ver PLAN C14)

## Seguridad

- No loguear passwords (ni en `tracing`, ni en log FTP — usar `PASS ***`).
- No commitear credenciales ni `sites.json` de usuario.
- Keyring service: `miel-ftp`, entrada por `connection.id` (UUID).

## Tests

- Tests unitarios en el mismo módulo con `#[cfg(test)]` (ver `config/store.rs`, `models/ftp_entry.rs`).
- Añadir tests cuando el cambio toque parsing, serialización o validación — no tests triviales.

## CI

GitHub Actions: `fmt --check`, `clippy -D warnings`, `test` — ver [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Reglas Cursor

Detalle por área en [`.cursor/rules/`](.cursor/rules/):

| Regla | Alcance |
|-------|---------|
| `project-core.mdc` | Siempre |
| `rust-standards.mdc` | `**/*.rs` |
| `iced-ui.mdc` | UI + `app.rs` |
| `ftp-layer.mdc` | `src/ftp/**` |
| `agent-workflow.mdc` | Siempre (git, calidad, comunicación) |

## Prioridad de implementación sugerida

1. Fase 2: actor `FtpSession`, streaming, progreso, cola
2. Fase 3: UX FileZilla (D&D, multi-select, breadcrumbs, …)
3. Fase 4: FTPS/SFTP, sync, throttling

Antes de Fase 4, confirmar con el usuario si SFTP entra en alcance (`suppaftp` no lo cubre).

## Comandos útiles

```bash
cargo run
RUST_LOG=miel_ftp=debug cargo run
cargo test
cargo fmt
cargo clippy -- -D warnings
```
