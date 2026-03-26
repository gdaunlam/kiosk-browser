# TODO — Kiosk Browser

## Pendiente

(Sin items pendientes)

## Completado

### Captura de teclado en Linux — backend evdev
- Implementado backend `evdev` + `uinput` que intercepta a nivel de dispositivo de entrada del kernel.
- Graba exclusivamente los dispositivos de teclado (`EVIOCGRAB`), filtra teclas bloqueadas y reenvía el resto via un teclado virtual (`uinput`).
- Funciona tanto en X11 como en Wayland, bypaseando completamente el Window Manager.
- Requiere permisos root o que el usuario pertenezca al grupo `input`.
- Si evdev falla (sin permisos), cae al backend X11 (`XGrabKey`) como fallback.
- El backend X11 original se mantiene como fallback para entornos sin acceso a `/dev/input`.

### Botón de cierre (close tab overlay)
- Implementado como overlay CSS/JS inyectado via `initialization_script`.
- Navegación a `kiosk://close` interceptada por `on_navigation` → `std::process::exit(0)`.
- No depende de IPC ni de `withGlobalTauri`.

### Carga de sitio con certificado TLS inválido (Linux)
- `TLSErrorsPolicy::Ignore` aplicado al `WebContext` real del webview (no al default).
- Se carga `index.html` primero para inicializar webkit, luego se navega al URL target via `load_uri`.

### Carga de sitio con certificado TLS inválido (Windows)
- WebView2 maneja errores TLS con su mecanismo nativo.

### Descarga de archivos
- Handler `on_download` extrae filename del URL y guarda en directorio de descargas del usuario.

### CI/CD
- GitHub Actions workflow para builds en Windows y Linux.

### Estructura del proyecto
- CLI con clap, logging con env_logger, módulos de keyboard por plataforma.
- READMEs separados por SO.
- .gitignore configurado.
