# Claude usage widget

Una cápsula para la barra de tareas de Windows que muestra el consumo del plan de Claude.

- Muestra solo los límites que tu plan informa: semanal, por modelo, 5 horas y uso extra.
- Cada límite se muestra u oculta desde el panel que se abre con un clic.
- Marca el ritmo esperado, para ver si vas por encima o por debajo de lo que corresponde a esta altura de la ventana.
- Ocho temas, y español o inglés.
- Se arrastra a lo largo de la barra y guarda la posición.
- Es un `.exe` nativo de menos de 1 MB.

## Usarlo

Abrí `claude-usage-widget.exe` y listo. En el primer arranque el widget se deja configurado: activa
el inicio con Windows y registra el mod que lo alimenta. Hace falta tener instalada la app de Claude.

El dato empieza a llegar con la próxima sesión de Claude Code que abras en la app.

## De dónde sale el dato

El widget no se conecta a ninguna red ni lee credenciales: dibuja lo que encuentra en
`%LOCALAPPDATA%\ClaudeUsageWidget\usage.json`.

Ese archivo lo escribe [`mod/`](mod), un mod de la app de Claude que cada minuto le pide los
límites a la propia app. Va dentro del `.exe`, que lo registra solo con el CLI de Claude Code. Por
eso el dato solo se actualiza mientras haya una sesión de Claude Code activa en la app; si no, el
widget muestra el último dato y avisa que no hay datos nuevos.

La consulta que usa el mod no es una API pública de Anthropic: si cambia, habrá que ajustar el mod.

## Compilar

No hace falta Visual Studio, solo Rust:

```bash
rustup target add x86_64-pc-windows-gnullvm
rustup component add rust-mingw --target x86_64-pc-windows-gnullvm
cargo test
cargo build --release
claude plugin test mod
```

La configuración está en `.cargo/config.toml`. Si el repo está en una carpeta sincronizada, como
OneDrive, conviene definir `build.target-dir` en `~/.cargo/config.toml`.
