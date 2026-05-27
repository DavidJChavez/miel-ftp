# MielFTP

Cliente FTP multiplataforma escrito en Rust con [Iced](https://iced.rs/). Interfaz oscura inspirada en Cursor / shadcn, orientada a paridad funcional con FileZilla.

El plan de desarrollo detallado está en **[PLAN.md](PLAN.md)** (auditoría, fases y decisiones de diseño).

Para agentes de Cursor y convenciones del repo, ver **[AGENTS.md](AGENTS.md)** y [`.cursor/rules/`](.cursor/rules/).

## Características

- Panel dual local / remoto con navegación por carpetas
- Site Manager: crear, editar y eliminar conexiones guardadas
- Contraseñas en el llavero del sistema (Keychain / Credential Manager)
- Subida y descarga de archivos con cola secuencial, progreso real y cancelación
- Log FTP opcional (mostrar / ocultar desde la barra inferior)
- Tema oscuro personalizado (acento miel)
- Logging estructurado con `tracing`

## Requisitos

- Rust 1.85+ (edition 2024)
- macOS, Linux o Windows

## Desarrollo

```bash
cargo run
```

Variables de entorno útiles:

```bash
RUST_LOG=miel_ftp=debug cargo run
```

## Tests y calidad

```bash
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## Configuración

Las conexiones se guardan en:

- **macOS / Linux:** `~/.config/miel-ftp/sites.json`
- **Windows:** `%APPDATA%\miel-ftp\sites.json`

Las contraseñas no se almacenan en JSON; van al keyring del sistema bajo el servicio `miel-ftp`.

## Roadmap

Leyenda: ✅ hecho · 🟡 parcial · ⬜ pendiente

### Núcleo y arquitectura

| Feature | Estado |
|---------|--------|
| Reducer iced (`boot` / `update` / `view`) | ✅ |
| Modelo de estado (`State`, `Message`) | ✅ |
| Errores tipados (`AppError`, `AppErrorMsg`) | ✅ |
| Capas desacopladas (`models` / `ftp` / `ui` / `config`) | ✅ |
| Validación de conexiones (`Connection::try_new`) | ✅ |
| Sesión FTP persistente (un stream por conexión) | ✅ |
| Timeouts y reintentos en operaciones de red | ✅ |

### Conexiones y configuración

| Feature | Estado |
|---------|--------|
| Site Manager (crear / editar / eliminar) | ✅ |
| Persistencia `sites.json` (escritura atómica) | ✅ |
| Contraseñas en keyring (`secrecy`) | ✅ |
| Quickconnect bar | ⬜ |
| FTPS (TLS explícito) | ⬜ |
| SFTP | ⬜ |
| Modo FTP activo / pasivo configurable | ⬜ |
| Bookmarks por sitio | ⬜ |

### Navegación de archivos

| Feature | Estado |
|---------|--------|
| Panel local (listar, subir carpeta, refrescar) | ✅ |
| Panel remoto (listar, navegar, refrescar) | ✅ |
| Parser LIST robusto (`suppaftp::list::File`) | ✅ |
| Rutas locales multiplataforma (`PathBuf`) | ✅ |
| Indicador de estado de conexión (sidebar) | ✅ |
| Breadcrumbs clickables | ✅ |
| Ordenar por columnas (nombre / tamaño / fecha) | ✅ |
| Filtro / búsqueda en carpeta | ✅ |
| Filtros globales (ocultar `.DS_Store`, etc.) | ✅ |
| Selección múltiple | ✅ |
| Menú contextual (rename, delete, mkdir, …) | ✅ |
| Atajos de teclado | ✅ |
| Drag & drop | ✅ |
| Mkdir / rename / delete (local y remoto) | ✅ |
| Comparar / sincronizar directorios | ⬜ |

### Transferencias

| Feature | Estado |
|---------|--------|
| Subir un archivo | ✅ |
| Bajar un archivo | ✅ |
| Barra de estado de transferencia (nombre + progreso real) | ✅ |
| Progreso real (bytes / %) vía `Task::stream` | ✅ |
| Streaming (sin cargar archivo entero en RAM) | ✅ |
| Cola de transferencias (pendiente / activa / historial) | ✅ |
| Transferencias concurrentes | ⬜ |
| Cancelar / reintentar | ✅ |
| Resume interrumpido (`REST`) | ✅ |
| Throttling de ancho de banda | ⬜ |
| Edición remota (temp + re-subida) | ⬜ |

### UI, UX y depuración

| Feature | Estado |
|---------|--------|
| Tema oscuro Miel (shadcn / Cursor-like) | ✅ |
| Modal Site Manager | ✅ |
| Banner de mensajes de estado | ✅ |
| Log FTP (comandos semánticos) | ✅ |
| Log FTP siempre oculto por defecto; toggle mostrar/ocultar | ✅ |
| Status bar (conteo archivos, tamaño total) | ✅ |
| Toasts / notificaciones | ⬜ |
| Iconos por tipo de archivo (SVG) | ✅ |
| Animaciones / skeleton loading | ⬜ |

### Calidad y proyecto

| Feature | Estado |
|---------|--------|
| Tests unitarios (config + parser LIST) | ✅ |
| CI (fmt, clippy, test) | ✅ |
| README y LICENSE | ✅ |
| Plan de desarrollo documentado ([PLAN.md](PLAN.md)) | ✅ |
| Cobertura de tests ampliada | ⬜ |

### Fases resumidas

| Fase | Descripción | Estado |
|------|-------------|--------|
| **1** | Bases sólidas, Site Manager, deuda técnica, log FTP opcional | ✅ |
| **2** | Sesión persistente, streaming, cola y progreso real | ✅ |
| **3** | Paridad UX FileZilla (D&D, multi-select, breadcrumbs, …) | ✅ |
| **4** | FTPS/SFTP, sync, edición remota, throttling | ⬜ |

## Licencia

MIT — ver [LICENSE](LICENSE).
