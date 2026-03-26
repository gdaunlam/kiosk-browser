# TODO — Kiosk Browser

## Pendiente

(Sin items pendientes)

## Completado

### Teclas bloqueadas llegan al sitio web (Linux)
- `XGrabKey` siempre se instala para capturar las teclas antes que el WM/compositor.
- Los eventos capturados se reenvían al webview como `KeyboardEvent` DOM sintéticos via `window.eval()`. Esto bypasea la cadena GTK → WebKitGTK que podía filtrar las teclas.
- Layer 1 (desactivación de atajos del WM via gsettings/kwriteconfig) se ejecuta primero para liberar los grabs pasivos del WM y evitar errores `BadAccess` en los nuestros.
- El sitio recibe keydown/keyup con las propiedades correctas (`altKey`, `metaKey`, `ctrlKey`, `shiftKey`, `key`, `code`).

### Fix hook de teclado en Windows
- El hook ahora bloquea tanto `WM_KEYDOWN`/`WM_SYSKEYDOWN` como `WM_KEYUP`/`WM_SYSKEYUP`. Esto corrige el Win key (el Start menu se activaba en key release).
- Se trackea el estado de Win key internamente con `AtomicBool` en lugar de `GetAsyncKeyState`, porque bloquear el keydown impedía que `GetAsyncKeyState` viera la tecla como presionada (y los combos Win+X fallaban).
- Se movió el overlay del botón de cierre de `top:0` a `top:10px` para evitar la dead zone de ~8px en Windows donde los eventos de mouse no llegan al webview (issue conocido de Tauri/WRY con `decorations:false`).

### Protección multicapa contra atajos del sistema (Linux)
- **Layer 0 — Tauri `prevent_close`**: `on_window_event` intercepta `CloseRequested` y llama `api.prevent_close()` cuando Alt+F4 está en la lista de teclas bloqueadas. Funciona en Windows y Linux.
- **Layer 1 — Desactivación de atajos del WM**: En KDE Plasma 6, usa `disableGlobalShortcuts` via D-Bus. En KDE Plasma 5, modifica `kglobalshortcutsrc` y `kwinrc` via `kwriteconfig5` y dispara `reconfigure` via D-Bus. En GNOME usa `gsettings`. En XFCE usa `xfconf-query`. Maneja correctamente el caso `sudo` (ejecuta `kwriteconfig` como el usuario original).
- **Layer 2 — X11 `XGrabKey`**: Grabs pasivos en la root window para interceptar eventos inyectados por VNC/xrdp que bypasean `/dev/input`.
- **Layer 3 — evdev `EVIOCGRAB`**: Graba dispositivos físicos de teclado a nivel kernel. Efectivo en bare-metal, no en VNC.

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
